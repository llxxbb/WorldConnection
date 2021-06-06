use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Sub};

use chrono::{Local, TimeZone};

use crate::db::{LastSelector, MetaCache, MetaDao, Relation};
use crate::db::downstream::DownStream;
use crate::db::flow_tool::{ContextChecker, StateChecker};
use crate::db::models::relation_target::RelationTarget;
use crate::domain::*;
use crate::util::*;

/// Control for how to generate next instance for downstream `Meta`
#[derive(Debug, Clone, Default)]
pub struct Mission {
    pub last_select: LastSelector,
    pub sys_context: HashMap<String, String>,
    pub downstream: DownStream,
}

impl Deref for Mission {
    type Target = DownStream;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.downstream
    }
}

impl DerefMut for Mission {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.downstream
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MissionRaw {
    pub to: String,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub last_select: LastSelector,
    pub executor: Executor,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub convert_before: Vec<Executor>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub convert_after: Vec<Executor>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub target_demand: RelationTarget,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub use_upstream_id: bool,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub delay: i32,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub sys_context: HashMap<String, String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub id_bridge: bool,
}

impl From<Mission> for MissionRaw {
    fn from(input: Mission) -> Self {
        MissionRaw {
            to: input.to.meta_string(),
            last_select: input.last_select.clone(),
            executor: input.executor.clone(),
            convert_before: input.convert_before.clone(),
            convert_after: input.convert_after.clone(),
            target_demand: input.target_demand.clone(),
            use_upstream_id: input.use_upstream_id,
            delay: input.delay,
            sys_context: input.sys_context.clone(),
            id_bridge: input.id_bridge,
        }
    }
}

impl MissionRaw {
    pub fn to_json(&self) -> Result<String> {
        let rtn = serde_json::to_string(self)?;
        Ok(rtn)
    }
    pub fn from_json(json: &str) -> Result<Self> {
        let rtn: Self = serde_json::from_str(json)?;
        Ok(rtn)
    }
}

pub type MissionFilter = fn(&Instance, &Vec<Relation>) -> Option<Vec<Mission>>;

impl Mission {
    pub fn for_dynamic(dynamic: Vec<DynamicConverter>) -> Result<Vec<Mission>> {
        debug!("------------------get_dynamic_route------------------------");
        let mut missions: Vec<Mission> = Vec::new();
        for d in dynamic {
            let t = match d.to {
                None => Meta::new("", 1, MetaType::Null)?,
                Some(s) => Meta::new(&s, 1, MetaType::Dynamic)?,
            };
            let mission = Mission {
                last_select: Default::default(),
                downstream: DownStream {
                    to: t,
                    executor: d.fun.clone(),
                    convert_before: vec![],
                    convert_after: vec![],
                    use_upstream_id: d.use_upstream_id,
                    target_demand: Default::default(),
                    delay: d.delay,
                    id_bridge: false,
                },
                sys_context: Default::default(),
            };
            missions.push(mission)
        }
        debug!("missions : {:?}", missions);
        Ok(missions)
    }

    /// Check the instance's context, sys_context and states whether satisfy the Selector request
    pub fn get_by_instance(instance: &Instance, relations: &Vec<Relation>, ctx_chk: ContextChecker, sta_chk: StateChecker) -> Vec<Mission> {
        if relations.is_empty() { return vec![]; }
        let mut rtn: Vec<Mission> = Vec::new();
        for r in relations {
            if r.selector.is_some() {
                let selector = &r.selector.clone().unwrap();
                if !ctx_chk(&instance.data.context, &selector.context_none, &selector.context_all, &selector.context_any) {
                    continue;
                }
                if !ctx_chk(&instance.data.sys_context, &selector.sys_context_none, &selector.sys_context_all, &selector.sys_context_any) {
                    continue;
                }
                // only verify source status, target status will be checked later.
                if !sta_chk(&instance.data.states, &selector.state_none, &selector.state_all, &selector.state_any) {
                    continue;
                }
            }
            let mut m = Mission::from(r.clone());
            if let Err(e) = init_by_instance(&mut m, &instance, r) {
                warn!("relation will be ignored, R: {}, E:{} ", r.relation_string(), e);
                continue;
            }
            // debug!("instance meta: {}, selected relation is {}", instance.meta, r.relation_string());
            rtn.push(m);
        }
        rtn
    }

    pub async fn from_raw<MC, M>(raw: &MissionRaw, mc_g: &MC, m_g: &M) -> Result<Self>
        where MC: MetaCache, M: MetaDao
    {
        let rtn = Mission {
            last_select: raw.last_select.clone(),
            downstream: DownStream {
                to: mc_g.get(&raw.to, m_g).await?,
                executor: raw.executor.clone(),
                convert_before: raw.convert_before.clone(),
                convert_after: raw.convert_after.clone(),
                use_upstream_id: raw.use_upstream_id,
                target_demand: raw.target_demand.clone(),
                delay: raw.delay,
                id_bridge: raw.id_bridge,
            },
            sys_context: raw.sys_context.clone(),
        };
        Ok(rtn)
    }
}

fn init_by_instance(m: &mut Mission, instance: &Instance, r: &Relation) -> Result<()> {
    m.delay = get_delay(instance, r)?;
    m.sys_context = instance.sys_context.clone();
    // replace para.dynamically. sys_context format : "[[\"p1\":\"v1\"],[\"p2\":\"v2\"]...]"
    match instance.sys_context.get(CONTEXT_DYNAMIC_PARA) {
        Some(paras) => {
            let paras: Vec<(String, String)> = serde_json::from_str(paras)?;
            if paras.is_empty() {
                return Ok(());
            }
            for para in paras {
                debug!("para dynamic will be replaced from {} to {} for relation: {:?}", para.0, para.1, r);
                m.executor.settings = m.executor.settings.replace(&para.0, &para.1);
                m.convert_before.iter_mut().for_each(|one| {
                    one.settings = one.settings.replace(&para.0, &para.1);
                });
                m.convert_after.iter_mut().for_each(|one| {
                    one.settings = one.settings.replace(&para.0, &para.1);
                });
            }
        }
        None => ()
    }
    Ok(())
}

impl From<Relation> for Mission {
    fn from(r: Relation) -> Self {
        let last_select = match &r.selector {
            None => LastSelector::default(),
            Some(sel) => LastSelector {
                last_all: sel.last_all.clone(),
                last_any: sel.last_any.clone(),
                last_none: sel.last_none.clone(),
            }
        };
        Mission {
            last_select,
            downstream: r.downstream.clone(),
            sys_context: Default::default(),
        }
    }
}

fn get_delay(ins: &Instance, rela: &Relation) -> Result<i32> {
    let rtn: i32 = if rela.delay > 0 {
        rela.delay
    } else if rela.delay_on_pare.0 > 0 {
        let rtn = get_para_and_key_from_para(&ins.para, &vec![rela.delay_on_pare.1])?;
        let diff = Local.timestamp_millis(rtn.0.parse::<i64>()?).sub(Local::now()).num_seconds();
        diff as i32 + rela.delay_on_pare.0
    } else {
        0
    };
    Ok(rtn)
}

#[cfg(test)]
mod test {
    use crate::db::flow_tool::{context_check, state_check};
    use crate::db::FlowSelector;
    use crate::db::models::relation_target::RelationTarget;

    use super::*;

    #[test]
    fn para_test() {
        assert_eq!("/a/b/c", "/a/${hello}/c".replace("${hello}", "b"));
        assert_eq!("/a/b/c", "/a/:hello:/c".replace(":hello:", "b"));
    }

    #[test]
    fn get_delay_test() {
        // none delay set
        let mut ins = Instance::default();
        let mut relation = Relation::default();
        let result = get_delay(&ins, &relation).unwrap();
        assert_eq!(result, 0);

        // para delay is set, but para not set
        relation.delay_on_pare = (100, 0);
        let result = get_delay(&ins, &relation);
        assert_eq!(result.is_err(), true);

        // para delay is set
        ins.para = (Local::now().timestamp_millis() + 200000).to_string();
        let result = get_delay(&ins, &relation).unwrap();
        assert_eq!(result >= 299 && result <= 300, true);

        // delay is set, delay is the high priority
        relation.delay = 50;
        ins.para = Local::now().timestamp_millis().to_string();
        let result = get_delay(&ins, &relation).unwrap();
        assert_eq!(result, 50);
    }

    #[test]
    fn state_verify() {
        let mut relation = Relation::default();
        let mut selector = FlowSelector::default();
        selector.state_any.insert("a".to_string());
        relation.selector = Some(selector);
        let relations = vec![relation];
        let mut instance = Instance::default();
        let rtn = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        assert_eq!(rtn.is_empty(), true);
        instance.states.insert("a".to_string());
        let rtn = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        assert_eq!(rtn.is_empty(), false);
    }

    #[test]
    fn sys_context_verify() {
        let mut relation = Relation::default();
        let mut selector = FlowSelector::default();
        selector.sys_context_any.insert("a".to_string());
        relation.selector = Some(selector);
        let relations = vec![relation];
        let mut instance = Instance::default();
        let rtn = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        assert_eq!(rtn.is_empty(), true);
        instance.sys_context.insert("a".to_string(), "x".to_string());
        let rtn = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        assert_eq!(rtn.is_empty(), false);
    }

    #[test]
    fn context_verify() {
        let mut relation = Relation::default();
        let mut selector = FlowSelector::default();
        selector.context_any.insert("a".to_string());
        relation.selector = Some(selector);
        let relations = vec![relation];
        let mut instance = Instance::default();
        let rtn = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        assert_eq!(rtn.is_empty(), true);
        instance.context.insert("a".to_string(), "x".to_string());
        let rtn = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        assert_eq!(rtn.is_empty(), false);
    }

    #[test]
    fn mission_copy_from_relation() {
        let meta = Meta::from_string("B:hello:1").unwrap();
        let executor = Executor::for_local("abc");
        let state = vec!["a".to_string()];
        let target = RelationTarget {
            state_add: state.clone(),
            state_remove: vec![],
            append_para: vec![],
            dynamic_para: "".to_string(),
        };
        let mut relation = Relation::default();
        relation.from = "a".to_string();
        relation.to = meta.clone();
        relation.executor = executor.clone();
        relation.use_upstream_id = true;
        relation.target_demand = target;
        relation.delay = 2;
        let relations = vec![relation];
        let rtn = Mission::get_by_instance(&Instance::default(), &relations, context_check, state_check);
        let rtn = &rtn[0];
        assert_eq!(rtn.delay, 2);
        assert_eq!(rtn.executor, executor);
        assert_eq!(rtn.to, meta);
        assert_eq!(rtn.use_upstream_id, true);
        assert_eq!(rtn.target_demand.state_add, state);
    }

    #[test]
    fn many_relations() {
        let relations = vec![Relation::default(), Relation::default(), Relation::default()];
        let rtn = Mission::get_by_instance(&Instance::default(), &relations, context_check, state_check);
        assert_eq!(rtn.len(), 3);
    }

    #[test]
    fn one_relation_but_no_selector() {
        let relations = vec![Relation::default()];
        let rtn = Mission::get_by_instance(&Instance::default(), &relations, context_check, state_check);
        assert_eq!(rtn.len(), 1);
    }

    #[test]
    fn no_relation() {
        let rtn = Mission::get_by_instance(&Instance::default(), &vec![], context_check, state_check);
        assert_eq!(rtn.is_empty(), true);
    }
}
