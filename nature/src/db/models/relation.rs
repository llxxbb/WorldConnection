use std::clone::Clone;
use std::ops::{Deref, DerefMut};
use std::string::ToString;

use crate::db::{MetaCache, MetaDao, RawRelation, RelationSettings, UpstreamSelector};
use crate::db::downstream::DownStream;
use crate::domain::*;

/// `form`,`selector`,`delay_on_pare` are the properties about upstream
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Relation {
    pub from: String,
    pub selector: Option<UpstreamSelector>,
    pub delay_on_pare: (i32, u8),
    pub downstream: DownStream,
}

impl Deref for Relation {
    type Target = DownStream;

    fn deref(&self) -> &Self::Target {
        &self.downstream
    }
}

impl DerefMut for Relation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.downstream
    }
}

impl Iterator for Relation {
    type Item = Relation;
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.clone())
    }
}

impl Relation {
    pub async fn from_raw<MC, M>(val: RawRelation, meta_cache_getter: &MC, meta_getter: &M) -> Result<Relation>
        where MC: MetaCache, M: MetaDao
    {
        let settings = match serde_json::from_str::<RelationSettings>(&val.settings) {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("{}'s setting format error: {:?}", val.get_string(), e);
                warn!("{}", &msg);
                return Err(NatureError::VerifyError(msg));
            }
        };
        let selector = &settings.selector;
        let m_to = Relation::check_converter(&val.to_meta, meta_cache_getter, meta_getter, &settings).await?;
        let rtn = match settings.executor {
            Some(e) => {
                // check Protocol type
                if e.protocol == Protocol::Auto {
                    let err = format!("{} Protocol::Auto can not be used by user. ", val.get_string());
                    return Err(NatureError::VerifyError(err));
                }
                Relation {
                    from: val.from_meta.to_string(),
                    downstream: DownStream {
                        to: m_to,
                        down_selector: settings.down_selector,
                        executor: e,
                        convert_before: settings.convert_before,
                        convert_after: settings.convert_after,
                        use_upstream_id: settings.use_upstream_id,
                        target_demand: settings.target.clone(),
                        delay: settings.delay,
                        id_bridge: settings.id_bridge,
                    },
                    selector: selector.clone(),
                    delay_on_pare: settings.delay_on_para,
                }
            }
            None => Relation {
                from: val.from_meta.to_string(),
                downstream: DownStream {
                    to: m_to.clone(),
                    down_selector: settings.down_selector,
                    executor: Executor::new_auto(),
                    convert_before: settings.convert_before,
                    convert_after: settings.convert_after,
                    use_upstream_id: settings.use_upstream_id,
                    target_demand: settings.target.clone(),
                    delay: settings.delay,
                    id_bridge: settings.id_bridge,
                },
                selector: selector.clone(),
                delay_on_pare: settings.delay_on_para,
            }
        };
        debug!("load {}", val.get_string());
        Ok(rtn)
    }

    async fn check_converter<MC, M>(meta_to: &str, meta_cache_getter: &MC, meta_getter: &M, settings: &RelationSettings) -> Result<Meta>
        where MC: MetaCache, M: MetaDao
    {
        let m_to = meta_cache_getter.get(meta_to, meta_getter).await?;
        let ts = &settings.target;
        if !&ts.state_add.is_empty() {
            Relation::check_state(&m_to, &ts.state_add)?
        };
        if !&ts.state_remove.is_empty() {
            Relation::check_state(&m_to, &ts.state_remove)?
        };
        Ok(m_to)
    }

    fn check_state(m_to: &Meta, x: &Vec<String>) -> Result<()> {
        let b = x.iter().filter(|one| { !m_to.has_state_name(one) }).collect::<Vec<&String>>();
        if b.len() > 0 {
            return Err(NatureError::VerifyError(format!("[to meta] did not defined state : {:?} ", b)));
        }
        Ok(())
    }

    pub fn relation_string(&self) -> String {
        format!("{}->{}", self.from, self.to.meta_string()).to_owned()
    }
}

#[cfg(test)]
mod test_from_raw {
    use tokio::runtime::Runtime;

    use crate::db::RawMeta;

    use super::*;

    #[test]
    fn master_should_have_relation() {
        let raw = RawRelation {
            id: 0,
            from_meta: "B:from:1".to_string(),
            to_meta: "B:to:1".to_string(),
            settings: "{}".to_string(),
            flag: 1,
        };
        let mg = MetaMock {};
        let mut rt = Runtime::new().unwrap();
        let rtn = rt.block_on(Relation::from_raw(raw, &MetaCacheMasterMock {}, &mg)).unwrap();
        assert_eq!(rtn.executor.protocol, Protocol::Auto);
    }

    #[test]
    fn setting_error_test() {
        let raw = RawRelation {
            id: 0,
            from_meta: "B:from:1".to_string(),
            to_meta: "B:to:1".to_string(),
            settings: "dd".to_string(),
            flag: 1,
        };
        let mg = MetaMock {};
        let mut rt = Runtime::new().unwrap();
        let rtn = rt.block_on(Relation::from_raw(raw, &MetaCacheMock {}, &mg));
        assert_eq!(rtn.err().unwrap().to_string().contains("relation[B:from:1  --->  B:to:1]"), true);
    }

    #[test]
    fn one_group_is_ok() {
        let settings = RelationSettings {
            selector: None,
            down_selector: None,
            executor: Some(Executor {
                protocol: Protocol::LocalRust,
                url: "url_one".to_string(),
                settings: "".to_string(),
            }),
            convert_before: vec![],
            convert_after: vec![],
            use_upstream_id: false,
            target: Default::default(),
            delay: 0,
            delay_on_para: (0, 0),
            id_bridge: false,
        };
        let raw = RawRelation {
            id: 0,
            from_meta: "B:from:1".to_string(),
            to_meta: "B:to:1".to_string(),
            settings: serde_json::to_string(&settings).unwrap(),
            flag: 1,
        };
        let mg = MetaMock {};
        let mut rt = Runtime::new().unwrap();
        let rtn = rt.block_on(Relation::from_raw(raw, &MetaCacheMock {}, &mg));
        assert_eq!(rtn.is_ok(), true);
    }

    #[derive(Copy, Clone)]
    struct MetaCacheMasterMock;

    #[async_trait]
    impl MetaCache for MetaCacheMasterMock {
        async fn get<M>(&self, m: &str, _getter: &M) -> Result<Meta> where M: MetaDao {
            if m.eq("B:to:1") {
                let mut rtn = Meta::from_string(m).unwrap();
                let _ = rtn.set_setting(r#"{"master":"B:from:1"}"#);
                Ok(rtn)
            } else {
                Meta::from_string(m)
            }
        }
    }

    #[derive(Copy, Clone)]
    struct MetaCacheMock;

    #[async_trait]
    impl MetaCache for MetaCacheMock {
        async fn get<M>(&self, meta_str: &str, _getter: &M) -> Result<Meta> where M: MetaDao {
            Meta::from_string(meta_str)
        }
    }

    #[derive(Copy, Clone)]
    struct MetaMock;

    #[async_trait]
    impl MetaDao for MetaMock {
        async fn get(&self, m: &str) -> Result<Option<RawMeta>> {
            Ok(Some(RawMeta::from(Meta::from_string(m)?)))
        }

        async fn insert(&self, _define: &RawMeta) -> Result<u64> {
            unimplemented!()
        }

        async fn update_flag(&self, _meta_str: &str, _flag_f: i32) -> Result<u64> {
            unimplemented!()
        }

        async fn edit(&self, _define: &RawMeta) -> Result<u64> {
            unimplemented!()
        }

        async fn delete(&self, _m: &Meta) -> Result<u64> {
            unimplemented!()
        }
        async fn id_great_than(&self, _from: i32, _limit: i32) -> Result<Vec<RawMeta>> {
            unimplemented!()
        }
    }
}