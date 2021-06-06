use std::collections::HashSet;

use crate::util::*;

/// select an upstream
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct UpstreamSelector {
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub state_all: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub state_any: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub state_none: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub context_all: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub context_any: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub context_none: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub sys_context_all: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub sys_context_any: HashSet<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub sys_context_none: HashSet<String>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn selector_serder_test() {
        let mut se = UpstreamSelector {
            state_all: HashSet::new(),
            state_any: Default::default(),
            state_none: HashSet::new(),
            context_all: HashSet::new(),
            context_any: Default::default(),
            context_none: HashSet::new(),
            sys_context_all: Default::default(),
            sys_context_any: Default::default(),
            sys_context_none: Default::default(),
        };
        // test for null
        let rtn = serde_json::to_string(&se);
        assert_eq!(rtn.unwrap(), "{}");

        // test for some value
        se.state_all.insert("aaa".to_string());
        let rtn = serde_json::to_string(&se).unwrap();
        assert_eq!(rtn, "{\"state_all\":[\"aaa\"]}");

        // deserialize

        let de: UpstreamSelector = serde_json::from_str(&rtn).unwrap();
        assert_eq!(de.context_none.is_empty(), true);
        assert_eq!(de.state_all.len(), 1);
    }
}
