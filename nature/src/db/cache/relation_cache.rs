use std::sync::Mutex;
use std::time::Duration;

use lru_time_cache::LruCache;

use crate::db::{MetaCache, MetaDao, Relation, RelationDao, Relations};
use crate::domain::*;

/// all flows for one upper `Meta` and what a chance to lower `group`
type ITEM = Vec<Relation>;
type CACHE = Mutex<LruCache<String, ITEM>>;
lazy_static! {
    pub static ref C_R: RelationCacheImpl = RelationCacheImpl {};
    static ref CACHE_MAPPING: CACHE = Mutex::new(LruCache::<String, ITEM>::with_expiry_duration(Duration::from_secs(3600)));
}

#[async_trait]
pub trait RelationCache {
    async fn get<R, MC, M>(&self, meta: &Meta, getter: &R, meta_cache: &MC, meta_dao: &M) -> Relations
        where R: RelationDao, MC: MetaCache, M: MetaDao;
}

pub struct RelationCacheImpl;

#[async_trait]
impl RelationCache for RelationCacheImpl {
    async fn get<R, MC, M>(&self, meta: &Meta, getter: &R, meta_cache: &MC, meta_dao: &M) -> Relations
        where R: RelationDao, MC: MetaCache, M: MetaDao {
        let meta_from: &str = &meta.meta_string();
        {
            let mut cache = CACHE_MAPPING.lock().unwrap();
            if let Some(rtn) = cache.get(meta_from) {
                return Ok(rtn.clone());
            }
        }
        let meta_type = meta.get_meta_type();
        if meta_type == MetaType::Multi || meta_type == MetaType::Loop {
            let msg = format!("MetaType::Multi && MetaType::Loop can't be used as `from` in `Relation`, the meta is: {}", meta_from);
            warn!("{}", msg);
            return Err(NatureError::VerifyError(msg));
        }
        debug!("-------- neta from : {}", meta_from);
        let rtn = getter.get_relations(meta_from, meta_cache, meta_dao).await?;
        if rtn.is_empty() {
            info!("no relation found for meta: {}", meta.get_key());
        }
        let cpy = rtn.clone();
        let mut cache = CACHE_MAPPING.lock().unwrap();
        cache.insert(meta_from.to_string(), rtn);
        Ok(cpy)
    }
}

#[cfg(test)]
mod test {
    use crate::db::{RawMeta, RawRelation};

    use super::*;

    #[tokio::test]
    async fn meta_type_is_multi_or_loop() {
        let from = Meta::from_string("M:error:1").unwrap();
        let result = C_R.get(&from, &RMockERR {}, &MCMock {}, &MetaMock {}).await;
        let error = result.err().unwrap().to_string();
        assert_eq!(true, error.contains("be used as"));

        let from = Meta::from_string("L:error:1").unwrap();
        let result = C_R.get(&from, &RMockERR2, &MCMock {}, &MetaMock {}).await;
        assert_eq!(true, result.err().unwrap().to_string().contains("be used as"));
    }

    #[tokio::test]
    async fn relation_error() {
        let from = Meta::from_string("B:error:1").unwrap();
        // this will call mocker
        let result = C_R.get(&from, &RMockERR {}, &MCMock {}, &MetaMock {}).await;
        assert_eq!(result, Err(NatureError::EnvironmentError("can't connect".to_string())));
        // error can't be cached
        let result = C_R.get(&from, &RMockERR2, &MCMock {}, &MetaMock {}).await;
        assert_eq!(result, Err(NatureError::EnvironmentError("another error".to_string())));
    }

    /// test cache also
    #[tokio::test]
    async fn get_none() {
        let from = Meta::from_string("B:none:1").unwrap();
        // this will call mocker
        let result = C_R.get(&from, &RMockNone {}, &MCMock {}, &MetaMock {}).await;
        assert_eq!(result.is_ok(), true);
        let result = result.unwrap();
        assert_eq!(result.is_empty(), true);
        // and the repeated call will not call mocker but get from cache
        let result = C_R.get(&from, &RMockERR {}, &MCMock {}, &MetaMock {}).await;
        assert_eq!(result.is_ok(), true);
        let result = result.unwrap();
        assert_eq!(result.is_empty(), true);
    }

    struct RMockERR;

    struct RMockERR2;

    struct RMockNone;

    #[async_trait]
    impl RelationDao for RMockERR {
        async fn get_relations<MC, M>(&self, _from: &str, _meta_cache_getter: &MC, _meta_getter: &M) -> Relations where MC: MetaCache, M: MetaDao {
            Err(NatureError::EnvironmentError("can't connect".to_string()))
        }

        async fn insert(&self, _one: RawRelation) -> Result<u64> {
            unimplemented!()
        }

        async fn delete(&self, _one: RawRelation) -> Result<u64> {
            unimplemented!()
        }

        async fn update_flag(&self, _from: &str, _to: &str, _flag_f: i32) -> Result<u64> {
            unimplemented!()
        }

        async fn insert_by_biz(&self, _from: &str, _to: &str, _url: &str, _protocol: &str) -> Result<RawRelation> {
            unimplemented!()
        }

        async fn delete_by_biz(&self, _from: &str, _to: &str) -> Result<u64> {
            unimplemented!()
        }
        async fn id_great_than(&self, _from: i32, _limit: i32) -> Result<Vec<RawRelation>> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl RelationDao for RMockERR2 {
        async fn get_relations<MC, M>(&self, _from: &str, _meta_cache_getter: &MC, _meta_getter: &M) -> Relations where MC: MetaCache, M: MetaDao {
            Err(NatureError::EnvironmentError("another error".to_string()))
        }

        async fn insert(&self, _one: RawRelation) -> Result<u64> {
            unimplemented!()
        }

        async fn delete(&self, _one: RawRelation) -> Result<u64> {
            unimplemented!()
        }

        async fn update_flag(&self, _from: &str, _to: &str, _flag_f: i32) -> Result<u64> {
            unimplemented!()
        }

        async fn insert_by_biz(&self, _from: &str, _to: &str, _url: &str, _protocol: &str) -> Result<RawRelation> {
            unimplemented!()
        }

        async fn delete_by_biz(&self, _from: &str, _to: &str) -> Result<u64> {
            unimplemented!()
        }
        async fn id_great_than(&self, _from: i32, _limit: i32) -> Result<Vec<RawRelation>> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl RelationDao for RMockNone {
        async fn get_relations<MC, M>(&self, _from: &str, _meta_cache_getter: &MC, _meta_getter: &M) -> Relations where MC: MetaCache, M: MetaDao {
            Ok(vec![])
        }

        async fn insert(&self, _one: RawRelation) -> Result<u64> {
            unimplemented!()
        }

        async fn delete(&self, _one: RawRelation) -> Result<u64> {
            unimplemented!()
        }

        async fn update_flag(&self, _from: &str, _to: &str, _flag_f: i32) -> Result<u64> {
            unimplemented!()
        }

        async fn insert_by_biz(&self, _from: &str, _to: &str, _url: &str, _protocol: &str) -> Result<RawRelation> {
            unimplemented!()
        }

        async fn delete_by_biz(&self, _from: &str, _to: &str) -> Result<u64> {
            unimplemented!()
        }
        async fn id_great_than(&self, _from: i32, _limit: i32) -> Result<Vec<RawRelation>> {
            unimplemented!()
        }
    }

    #[derive(Copy, Clone)]
    struct MCMock;

    #[async_trait]
    impl MetaCache for MCMock {
        async fn get<M>(&self, meta_str: &str, _getter: &M) -> Result<Meta> where M: MetaDao {
            Ok(Meta::from_string(meta_str)?)
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
