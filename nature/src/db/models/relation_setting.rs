use crate::db::{DownstreamSelector, UpstreamSelector};
use crate::db::relation_target::RelationTarget;
use crate::domain::*;
use crate::util::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RelationSettings {
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub selector: Option<UpstreamSelector>,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub down_selector: Option<DownstreamSelector>,
    /// array executor will share the convert task
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub executor: Option<Executor>,
    /// filter will execute before executor,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub convert_before: Vec<Executor>,
    /// filter will execute after executor,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub convert_after: Vec<Executor>,
    /// if the downstream is state meta, when `is_main` is set to true, the upstream's id will be used as downstream's id
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub use_upstream_id: bool,
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub target: RelationTarget,
    /// delay seconds to execute the converter
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub delay: i32,
    /// delay seconds to execute the converter based on `Instance.para`
    /// - first value : how long to delay
    /// - second value : time part based on of the `Instance.para`
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub delay_on_para: (i32, u8),
    /// if this relation's next target will use the upstream's id please set this property to `true`
    #[serde(skip_serializing_if = "is_default")]
    #[serde(default)]
    pub id_bridge: bool,
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn default_test() {
        let settings = RelationSettings::default();
        let result = serde_json::to_string(&settings).unwrap();
        assert_eq!(result, "{}");
    }

    #[test]
    fn delay_on_para() {
        let mut settings = RelationSettings::default();
        settings.delay_on_para = (100, 20);
        let result = serde_json::to_string(&settings).unwrap();
        assert_eq!(result, r#"{"delay_on_para":[100,20]}"#);
        let rtn = serde_json::from_str::<RelationSettings>(&result).unwrap();
        assert_eq!(rtn, settings);
    }

    #[test]
    fn selector_test() {
        let mut set = HashSet::<String>::new();
        set.insert("one".to_string());
        let mut fs = UpstreamSelector::default();
        fs.state_all = set;

        let mut setting = RelationSettings::default();
        setting.selector = Some(fs);

        let result = serde_json::to_string(&setting).unwrap();
        let res_str = r#"{"selector":{"state_all":["one"]}}"#;
        assert_eq!(result, res_str);
        let res_obj: RelationSettings = serde_json::from_str(res_str).unwrap();
        assert_eq!(res_obj, setting);
    }

    #[test]
    fn empty_executor_test() {
        let setting = RelationSettings::default();
        let result = serde_json::to_string(&setting).unwrap();
        let res_str = r#"{}"#;
        assert_eq!(result, res_str);
        let res_obj: RelationSettings = serde_json::from_str(res_str).unwrap();
        assert_eq!(res_obj, setting);
    }

    #[test]
    fn executor_test() {
        let executor = Executor {
            protocol: Protocol::LocalRust,
            url: "nature-demo:order_new".to_string(),
            settings: "".to_string(),
        };
        let mut setting = RelationSettings::default();
        setting.executor = Some(executor);
        let result = serde_json::to_string(&setting).unwrap();
        let res_str = r#"{"executor":{"protocol":"localRust","url":"nature-demo:order_new"}}"#;
        assert_eq!(result, res_str);
        let res_obj: RelationSettings = serde_json::from_str(res_str).unwrap();
        assert_eq!(res_obj, setting);
    }

    #[test]
    fn use_upstream_id() {
        let mut setting = RelationSettings::default();
        setting.use_upstream_id = true;
        let result = serde_json::to_string(&setting).unwrap();
        let res_str = r#"{"use_upstream_id":true}"#;
        assert_eq!(result, res_str);
        let res_obj: RelationSettings = serde_json::from_str(res_str).unwrap();
        assert_eq!(res_obj, setting);
    }

    #[test]
    fn target_state() {
        let state = vec!["new".to_string()];
        let mut setting = RelationSettings::default();
        setting.target.state_add = state;
        let result = serde_json::to_string(&setting).unwrap();
        let res_str = r#"{"target":{"state_add":["new"]}}"#;
        assert_eq!(result, res_str);
        let res_obj: RelationSettings = serde_json::from_str(res_str).unwrap();
        assert_eq!(res_obj, setting);
    }
}