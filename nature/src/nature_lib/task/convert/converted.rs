use crate::db::{Mission, RawTask};
use crate::domain::*;
use crate::nature_lib::task::{CachedKey, TaskForConvert};
use crate::util::*;

pub struct Converted {
    pub done_task: RawTask,
    pub converted: Vec<Instance>,
}

impl Converted {
    pub fn gen(task: &TaskForConvert, convert_task: &RawTask, instances: Vec<Instance>, last_state: &Option<Instance>) -> Result<Converted> {
        if instances.is_empty() {
            return Ok(converted_none(convert_task));
        }

        let mut instances = instances;

        // init meta and [from]
        let from = FromInstance::from(&task.from);
        let _ = set_source_and_target_meta(&mut instances, &from, &task.target.to)?;

        // check id
        let _ = check_id(&mut instances, &from, &task.target)?;

        // filter from cache
        let mut instances: Vec<Instance> = if task.check_cache() {
            instances.into_iter().filter(|one| !CachedKey::get(&one.get_key())).collect()
        } else {
            instances
        };

        // verify state
        let _ = verify_state(&task, &mut instances, last_state)?;

        if task.target.id_bridge {
            bridge_context_id(&mut instances, &task.target, &from);
            bridge_context_para(&mut instances, &task.target, &from);
        }

        // assemble it
        let rtn = Converted {
            done_task: convert_task.to_owned(),
            converted: instances,
        };
        Ok(rtn)
    }
}

fn converted_none(carrier: &RawTask) -> Converted {
    Converted {
        done_task: carrier.to_owned(),
        converted: Vec::new(),
    }
}

fn set_source_and_target_meta(instances: &mut Vec<Instance>, from: &FromInstance, target_meta: &Meta) -> Result<()> {
    match target_meta.get_meta_type() {
        MetaType::Multi => {
            match target_meta.get_setting() {
                Some(s) => s.check_multi_meta(instances, from)?,
                None => return Err(NatureError::LogicalError("MetaType::Multi must has settings".to_string())),
            }
        }
        MetaType::Loop => {
            match target_meta.get_setting() {
                Some(s) => s.check_multi_meta(instances, from)?,
                None => return Err(NatureError::LogicalError("MetaType::Loop must has settings".to_string())),
            }
        }
        _ => set_all_instances(instances, from, target_meta),
    }
    Ok(())
}

fn set_all_instances(instances: &mut Vec<Instance>, from: &FromInstance, target_meta: &Meta) {
    instances.iter_mut().for_each(|n| {
        n.data.meta = target_meta.meta_string();
        n.from = Some(from.clone());
    });
}

fn check_id(ins: &mut Vec<Instance>, from: &FromInstance, target: &Mission) -> Result<()> {
    if ins.is_empty() {
        return Ok(());
    }
    // handle state-instance, the id and para had put to the sys_context before convert
    if target.to.is_state() {
        if let Some(id) = target.sys_context.get(CONTEXT_TARGET_INSTANCE_ID) {
            ins[0].id = id.to_string();
        }
        if let Some(para) = target.sys_context.get(CONTEXT_TARGET_INSTANCE_PARA) {
            ins[0].para = para.to_string();
        }
        ins[0].revise()?;
        // set sys_context
        if !target.target_demand.dynamic_para.is_empty() {
            let para = &ins[0].para.to_string();
            append_dynamic_para_from_mission(target, &mut &mut ins[0], &para)?
        }
        return Ok(());
    }

    // handle normal-instance
    let id = {
        if target.use_upstream_id || target.to.check_master(&from.meta) {
            Some(from.id.to_string())
        } else { None }
    };

    for mut one in ins {
        if target.target_demand.append_para.len() > 0 {
            let result = get_para_and_key_from_para(&from.para, &target.target_demand.append_para)?;
            one.para = append_para(&one.para, &result.0);
            // set sys_context
            if !target.target_demand.dynamic_para.is_empty() {
                append_dynamic_para_from_mission(target, &mut one, &result.0)?
            }
        }
        if let Some(ref id_u) = id {
            one.id = id_u.to_string();
        }
        one.revise()?;
    }
    Ok(())
}

fn append_dynamic_para_from_mission(target: &Mission, one: &mut &mut Instance, value: &str) -> Result<()> {
    let option = one.sys_context.get(CONTEXT_DYNAMIC_PARA);
    let mut paras = match option {
        Some(s) => serde_json::from_str::<Vec<(String, String)>>(s)?,
        None => vec![]
    };
    paras.push((target.target_demand.dynamic_para.clone(), value.to_string()));
    one.sys_context.insert(CONTEXT_DYNAMIC_PARA.to_string(), serde_json::to_string(&paras)?);
    Ok(())
}

fn bridge_context_id(instances: &mut Vec<Instance>, mission: &Mission, from: &FromInstance) {
    if let Some(id) = mission.sys_context.get(CONTEXT_TARGET_INSTANCE_ID) {
        for instance in instances {
            instance.data.sys_context.insert(CONTEXT_TARGET_INSTANCE_ID.to_string(), id.to_string());
        }
    } else {
        for instance in instances {
            instance.data.sys_context.insert(CONTEXT_TARGET_INSTANCE_ID.to_string(), format!("{}", from.id));
        }
    }
}

fn bridge_context_para(instances: &mut Vec<Instance>, mission: &Mission, from: &FromInstance) {
    if let Some(para) = mission.sys_context.get(CONTEXT_TARGET_INSTANCE_PARA) {
        for instance in instances {
            instance.data.sys_context.insert(CONTEXT_TARGET_INSTANCE_PARA.to_string(), para.to_string());
        }
    } else if !from.para.is_empty() {
        for instance in instances {
            instance.data.sys_context.insert(CONTEXT_TARGET_INSTANCE_PARA.to_string(), from.para.to_string());
        }
    }
}

fn verify_state(task: &TaskForConvert, instances: &mut Vec<Instance>, last_state: &Option<Instance>) -> Result<()> {
    let to = &task.target.to;
    if !to.is_state() {
        return Ok(());
    }
    if instances.len() > 1 {
// only one status instance should return
        return Err(NatureError::LogicalError("[status meta] must return less 2 instances!".to_string()));
    }
    let ins = &mut instances[0];

// states and state version
    let temp_states = ins.states.clone();
    match last_state {
        None => {
            if task.from.meta == task.target.to.meta_string() {
                ins.state_version = task.from.state_version + 1;
            } else {
                ins.state_version = 1;
            }
        }
        Some(x) => {
            ins.state_version = x.state_version + 1;
            ins.states = x.states.clone();
        }
    };
// set status
    let lsd = &task.target.target_demand;
    if !lsd.state_remove.is_empty() || !lsd.state_add.is_empty() {
        ins.modify_state(lsd, &task.target.to);
    } else {
        let (_, mutex) = task.target.to.check_state(&temp_states.clone().into_iter().collect())?;
        if mutex.len() > 0 {
            return Err(NatureError::LogicalError(format!("returned mutex state {:?}", mutex)));
        }
        ins.states = temp_states
    }
    Ok(())
}

#[cfg(test)]
mod sys_context_test {
    use super::*;

    #[test]
    fn id_from_previous_id() {
        let mut ins: Vec<Instance> = vec![Instance::default()];
        let mut mission = Mission::default();
        mission.id_bridge = true;
        let mut from = FromInstance::default();
        from.id = "123".to_string();
        bridge_context_id(&mut ins, &mission, &from);
        assert_eq!("123", ins[0].sys_context.get("target.id").unwrap());
    }

    #[test]
    fn id_from_previous_context() {
        let mut ins: Vec<Instance> = vec![Instance::default()];
        let mut mission = Mission::default();
        mission.sys_context.insert("target.id".to_string(), "abc".to_string());
        mission.id_bridge = true;
        bridge_context_id(&mut ins, &mission, &FromInstance::default());
        assert_eq!("abc", ins[0].sys_context.get("target.id").unwrap());
    }
}

#[cfg(test)]
mod test {
    use chrono::Local;

    use crate::db::downstream::DownStream;
    use crate::db::relation_target::RelationTarget;

    use super::*;

    #[test]
    fn upstream_test() {
        let mut from_ins = Instance::default();
        from_ins.id = "567".to_string();
        from_ins.meta = "B:from:1".to_string();
        from_ins.state_version = 2;
        let meta = Meta::new("to", 1, MetaType::Business).unwrap();
        let task_key = from_ins.get_key();
        let task = TaskForConvert {
            from: from_ins,
            target: Mission {
                last_select: Default::default(),
                downstream: DownStream {
                    to: meta.clone(),
                    executor: Default::default(),
                    convert_before: vec![],
                    convert_after: vec![],
                    use_upstream_id: true,
                    target_demand: Default::default(),
                    delay: 0,
                    id_bridge: false,
                },
                sys_context: Default::default(),
            },
            conflict_version: 0,
        };
        let raw = RawTask {
            task_id: 0,
            task_key,
            task_type: 0,
            task_for: "".to_string(),
            data: "".to_string(),
            create_time: Local::now().naive_local(),
            execute_time: Local::now().naive_local(),
            retried_times: 0,
            task_state: 0,
        };
        let mut ins = Instance::default();
        ins.id = "123".to_string();
        let ins = vec![ins];

        let result = Converted::gen(&task, &raw, ins.clone(), &None).unwrap();
        let c = &result.converted[0];
        let from = c.from.as_ref().unwrap();
        assert_eq!(from.id, "567".to_string());
        assert_eq!(from.meta, "B:from:1");
        assert_eq!(from.state_version, 2);
        assert_eq!(result.converted[0].id, "567".to_string());
    }

    #[test]
    fn target_states_test() {
        let task = TaskForConvert {
            from: Default::default(),
            target: Mission {
                last_select: Default::default(),
                downstream: DownStream {
                    to: {
                        let mut m = Meta::from_string("B:hello:1").unwrap();
                        let _ = m.set_states(Some(vec![State::Normal("new".to_string())]));
                        m
                    },
                    executor: Default::default(),
                    convert_before: vec![],
                    convert_after: vec![],
                    use_upstream_id: false,
                    target_demand: RelationTarget {
                        state_add: vec!["new".to_string()],
                        state_remove: vec![],
                        append_para: vec![],
                        dynamic_para: "".to_string(),
                    },
                    delay: 0,
                    id_bridge: false,
                },
                sys_context: Default::default(),
            },
            conflict_version: 0,
        };
        let mut ins = vec![Instance::new("test").unwrap()];
        let _ = verify_state(&task, &mut ins, &None);
        let one = &ins[0];
        assert_eq!(one.states.contains("new"), true)
    }
}

#[cfg(test)]
mod check_id_for_state {
    use super::*;

    #[test]
    fn input_is_empty() {
        let (from, mission) = init_input();
        let rtn = check_id(&mut vec![], &from, &mission);
        assert_eq!(rtn.is_ok(), true)
    }

    #[test]
    fn sys_context_nothing() {
        let (from, mission) = init_input();
        let sta = Instance::default();
        let vec = &mut vec![sta];
        assert_eq!(vec[0].id, "");
        let rtn = check_id(vec, &from, &mission);
        assert_eq!(rtn.is_ok(), true);
        assert_eq!(vec[0].id.len() > 0, true)
    }

    #[test]
    fn sys_context_for_id() {
        let (from, mut mission) = init_input();
        let sta = Instance::default();
        mission.sys_context.insert("target.id".to_string(), "5".to_string());
        let vec = &mut vec![sta];
        assert_eq!(vec[0].id, "");
        let rtn = check_id(vec, &from, &mission);
        assert_eq!(rtn.is_ok(), true);
        assert_eq!(vec[0].id, "5")
    }

    #[test]
    fn only_para_for_context() {
        let (from, mut mission) = init_input();
        let sta = Instance::default();
        mission.sys_context.insert("target.para".to_string(), "a".to_string());
        let vec = &mut vec![sta];
        let rtn = check_id(vec, &from, &mission);
        assert_eq!(rtn.is_ok(), true);
        assert_eq!(vec[0].id, "");
        assert_eq!(vec[0].para, "a")
    }

    /// master is from
    fn init_input() -> (FromInstance, Mission) {
        let from = FromInstance::default();

        let mut setting = MetaSetting::default();
        setting.is_state = true;

        let mut meta = Meta::new("to", 1, MetaType::Business).unwrap();
        let _ = meta.set_setting(&setting.to_json().unwrap());

        let mut mission = Mission::default();
        mission.to = meta;

        (from, mission)
    }
}

#[cfg(test)]
mod check_id_for_normal {
    use super::*;

    #[test]
    fn normal() {
        let (from, mut mission) = init_input();
        mission.to = Meta::new("remove master", 1, MetaType::Business).unwrap();
        let one = Instance::new("one").unwrap();
        let mut input = vec![one];
        let _ = check_id(&mut input, &from, &mission);
        assert_eq!(input[0].id != "123", true);
        assert_eq!(input[0].id != "0", true);
    }

    #[test]
    fn append_para() {
        let (mut from, mut mission) = init_input();
        from.para = "c/d/e".to_string();
        mission.to = Meta::new("remove master", 1, MetaType::Business).unwrap();
        mission.target_demand.append_para = vec![0, 2];
        mission.target_demand.dynamic_para = "(a)".to_string();
        let mut one = Instance::new("one").unwrap();
        one.para = "a/b".to_string();
        let mut input = vec![one];
        let _ = check_id(&mut input, &from, &mission);
        assert_eq!(input[0].para, "a/b/c/e");
        assert_eq!(input[0].sys_context.get(CONTEXT_DYNAMIC_PARA).unwrap(), "[[\"(a)\",\"c/e\"]]");
    }

    #[test]
    fn use_upstream_id() {
        let (from, mut mission) = init_input();
        mission.to = Meta::new("remove master", 1, MetaType::Business).unwrap();
        mission.use_upstream_id = true;
        let one = Instance::new("one").unwrap();
        let mut input = vec![one];
        let _ = check_id(&mut input, &from, &mission);
        assert_eq!(input[0].id, "123");
    }


    /// the instance that will be saved is child of from
    #[test]
    fn use_master_id() {
        let (from, mission) = init_input();
        let one = Instance::new("one").unwrap();
        let mut input = vec![one];
        let _ = check_id(&mut input, &from, &mission);
        assert_eq!(input[0].id, "123");
    }

    /// master is from
    fn init_input() -> (FromInstance, Mission) {
        let from = FromInstance {
            id: "123".to_string(),
            meta: "from".to_string(),
            para: "".to_string(),
            state_version: 1,
        };

        let mut setting = MetaSetting::default();
        setting.master = Some("from".to_string());

        let mut meta = Meta::new("to", 1, MetaType::Business).unwrap();
        let _sr = meta.set_setting(&setting.to_json().unwrap());

        let mut mission = Mission::default();
        mission.to = meta;
        (from, mission)
    }
}