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

#[test]
fn an_account_with_an_unknown_provider_is_an_error_not_a_crash() {
    let harness = Harness::new();
    let core = harness.open();
    harness.arrange_local_store("INSERT INTO account (provider) VALUES ('exchange');");

    // Asked twice: a failed read must leave the core usable.
    for _ in 0..2 {
        assert!(matches!(
            core.list_accounts(),
            Err(CoreError::UnknownProvider(provider)) if provider == "exchange"
        ));
    }
}
