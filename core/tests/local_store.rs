mod support;

use freecal_core::CoreError;
use support::Harness;

#[test]
fn a_local_store_from_a_newer_freecal_is_refused() {
    let harness = Harness::new();
    harness.arrange_local_store("PRAGMA user_version = 99;");

    let result = harness.try_open();

    assert!(matches!(
        result,
        Err(CoreError::NewerLocalStore { found: 99, .. })
    ));
}
