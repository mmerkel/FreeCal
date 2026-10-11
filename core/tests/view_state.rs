mod support;

use freecal_core::View;
use support::Harness;

#[test]
fn freecal_first_opens_in_month_view() {
    let harness = Harness::new();
    let core = harness.open();

    assert_eq!(core.last_used_view().unwrap(), View::Month);
}

#[test]
fn the_last_used_view_is_kept_across_launches() {
    let harness = Harness::new();
    {
        let core = harness.open();
        core.set_last_used_view(View::Week).unwrap();
    }

    let core = harness.open();

    assert_eq!(core.last_used_view().unwrap(), View::Week);
}

#[test]
fn an_unknown_stored_view_falls_back_to_month() {
    let harness = Harness::new();
    let core = harness.open();
    harness.arrange_local_store("UPDATE app_state SET view = 'year';");

    assert_eq!(core.last_used_view().unwrap(), View::Month);
}
