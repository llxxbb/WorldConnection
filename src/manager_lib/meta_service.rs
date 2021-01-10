use crate::db::{D_M, MetaDao, QUERY_SIZE_LIMIT, RawMeta};
use crate::domain::*;

pub struct MetaService {}

impl MetaService {
    pub async fn id_great_than(from: i32, limit: i32) -> Result<Vec<RawMeta>> {
        let limit = if limit < *QUERY_SIZE_LIMIT {
            limit
        } else { *QUERY_SIZE_LIMIT };
        D_M.id_great_than(from, limit).await
    }
}