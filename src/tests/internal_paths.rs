#[test]
fn config_root_errors_when_required_environment_is_missing() {
    let err = crate::paths::test_config_root_from(|_| None).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
