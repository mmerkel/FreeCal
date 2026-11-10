mod support;

use freecal_core::{Appearance, Settings, SettingsUpdate};
use support::Harness;

#[test]
fn freecal_first_follows_the_systems_appearance() {
    let harness = Harness::new();
    let core = harness.open();

    assert_eq!(
        core.settings().unwrap(),
        Settings {
            appearance: Appearance::System,
            run_in_background: true,
            start_at_login: false,
        }
    );
}

#[test]
fn a_changed_setting_is_kept_across_launches() {
    let harness = Harness::new();
    {
        let core = harness.open();
        let changed = core
            .update_settings(SettingsUpdate {
                appearance: Some(Appearance::Dark),
                ..SettingsUpdate::default()
            })
            .unwrap();
        assert_eq!(changed.appearance, Appearance::Dark);
    }

    let core = harness.open();

    assert_eq!(core.settings().unwrap().appearance, Appearance::Dark);
}

#[test]
fn an_update_changes_only_the_settings_it_gives() {
    let harness = Harness::new();
    let core = harness.open();
    core.update_settings(SettingsUpdate {
        appearance: Some(Appearance::Light),
        ..SettingsUpdate::default()
    })
    .unwrap();

    let unchanged = core.update_settings(SettingsUpdate::default()).unwrap();

    assert_eq!(unchanged.appearance, Appearance::Light);
    assert_eq!(core.settings().unwrap().appearance, Appearance::Light);
}

#[test]
fn an_update_from_the_frontend_may_leave_out_any_setting() {
    let update: SettingsUpdate = serde_json::from_str("{}").unwrap();

    assert_eq!(update, SettingsUpdate::default());
}

#[test]
fn an_unknown_stored_appearance_falls_back_to_the_system() {
    let harness = Harness::new();
    let core = harness.open();
    harness.arrange_local_store("UPDATE app_state SET appearance = 'sepia';");

    assert_eq!(core.settings().unwrap().appearance, Appearance::System);
}

#[test]
fn background_running_and_start_at_login_are_kept_across_launches() {
    let harness = Harness::new();
    {
        let core = harness.open();
        let changed = core
            .update_settings(SettingsUpdate {
                run_in_background: Some(false),
                start_at_login: Some(true),
                ..SettingsUpdate::default()
            })
            .unwrap();
        assert!(!changed.run_in_background);
        assert!(changed.start_at_login);
    }

    let settings = harness.open().settings().unwrap();

    assert!(!settings.run_in_background);
    assert!(settings.start_at_login);
    assert_eq!(settings.appearance, Appearance::System);
}

#[test]
fn unknown_stored_switches_fall_back_to_their_defaults() {
    let harness = Harness::new();
    let core = harness.open();
    harness
        .arrange_local_store("UPDATE app_state SET run_in_background = 'no', start_at_login = 7;");

    let settings = core.settings().unwrap();

    assert!(settings.run_in_background);
    assert!(!settings.start_at_login);
}
