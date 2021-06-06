use std::collections::HashSet;

use crate::util::*;

/// Convenient for store task info when serializing `Mission`
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct DownstreamSelector {
    // The state of downstream, required all
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub last_all: HashSet<String>,
    // The state of downstream, required any
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub last_any: HashSet<String>,
    // The state of downstream, required none
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub last_none: HashSet<String>,
}