mod support;

use freecal_core::{CoreError, Provider};
use support::Harness;

#[test]
fn the_local_account_exists_on_first_launch() {
    let harness = Harness::new();
    let core = harness.open();

    let accounts = core.list_accounts().unwrap();

    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].provider, Provider::Local);
}

#[test]
fn relaunching_keeps_the_same_single_local_account() {
    let harness = Harness::new();
    let first_launch = harness.open().list_accounts().unwrap();

    let relaunch = harness.open().list_accounts().unwrap();

    assert_eq!(relaunch, first_launch);
}

#[test]
fn the_local_account_cannot_be_removed() {
    let harness = Harness::new();
    let core = harness.open();
    let local = core.list_accounts().unwrap()[0].id;

    let result = core.remove_account(local);

    assert!(matches!(
        result,
        Err(CoreError::LocalAccountCannotBeRemoved)
    ));
    assert_eq!(core.list_accounts().unwrap().len(), 1);
}
