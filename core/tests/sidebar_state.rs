mod support;

use freecal_core::SidebarState;
use support::Harness;

#[test]
fn the_sidebar_first_shows_at_its_default_width() {
    let harness = Harness::new();
    let core = harness.open();

    assert_eq!(
        core.sidebar().unwrap(),
        SidebarState {
            shown: true,
            width: None
        }
    );
}

#[test]
fn the_sidebar_state_is_kept_across_launches() {
    let harness = Harness::new();
    let hidden = SidebarState {
        shown: false,
        width: Some(18.5),
    };
    {
        let core = harness.open();
        core.set_sidebar(hidden).unwrap();
    }

    let core = harness.open();

    assert_eq!(core.sidebar().unwrap(), hidden);
}

#[test]
fn an_unreadable_stored_width_falls_back_to_the_default() {
    let harness = Harness::new();
    let core = harness.open();

    for stored in ["'wide'", "-3", "0", "X'00'"] {
        harness.arrange_local_store(&format!("UPDATE app_state SET sidebar_width = {stored};"));
        assert_eq!(core.sidebar().unwrap().width, None, "{stored}");
    }
}

#[test]
fn an_unreadable_stored_shown_flag_shows_the_sidebar() {
    let harness = Harness::new();
    let core = harness.open();

    for stored in ["'no'", "2.5", "X'01'"] {
        harness.arrange_local_store(&format!("UPDATE app_state SET sidebar_shown = {stored};"));
        assert!(core.sidebar().unwrap().shown, "{stored}");
    }
}
