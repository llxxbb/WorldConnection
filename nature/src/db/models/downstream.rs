use crate::domain::{Meta, Executor};
use crate::db::models::relation_target::RelationTarget;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DownStream {
    pub to: Meta,
    pub executor: Executor,
    pub convert_before: Vec<Executor>,
    pub convert_after: Vec<Executor>,
    pub use_upstream_id: bool,
    pub target_demand: RelationTarget,
    pub delay: i32,
    pub id_bridge: bool,
}
