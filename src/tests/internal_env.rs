use super::support::with_env_vars;

#[test]
fn env_object_override_replaces_scalar_intermediate_node() {
    with_env_vars(&[("APP_DATABASE__URL", "\"postgres://override\"")], || {
        let value = serde_json::json!({
            "database": "from-default"
        });

        let merged: serde_json::Value = crate::env::apply_env_overrides(value, "APP_").unwrap();

        assert_eq!(
            merged,
            serde_json::json!({
                "database": {
                    "url": "postgres://override"
                }
            })
        );
    });
}
