#[test]
fn internal_api_key_env_name_is_stable() {
    let cfg = payments_gateway::config::AppConfig::from_env();
    assert!(!cfg.internal_api_key.is_empty());
}

#[test]
fn readiness_endpoints_exist_in_readme() {
    let readme = std::fs::read_to_string("README.md").unwrap_or_default();
    assert!(readme.contains("/ops/readiness"));
    assert!(readme.contains("/ops/liveness"));
    assert!(readme.contains("/bandit/policy/:segment/disable"));
}
