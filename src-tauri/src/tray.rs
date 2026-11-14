//! FreeCal's tray icon: a StatusNotifierItem through Tauri, which KDE Plasma
//! shows natively (US-108). A left click opens the window; a right click
//! opens the menu. Linux gives the icon clicks only through the KSNI backend
//! of Tauri's tray crate, which `Cargo.toml` turns on. The icon's status and
//! Sync now are tickets 23 and 16.

use std::sync::Arc;
use tauri::AppHandle;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

/// The tray icon's ID, by which the shell finds it.
pub const TRAY: &str = "freecal";

/// What the user picked in the tray menu.
pub enum TrayChoice {
    OpenFreeCal,
    Quit,
}

const OPEN_FREECAL: &str = "open-freecal";
const QUIT: &str = "quit";

/// Puts FreeCal's icon in the tray, with `on_choice` for its menu and its
/// left click, which picks Open FreeCal.
pub fn show(
    app: &AppHandle,
    on_choice: impl Fn(&AppHandle, TrayChoice) + Send + Sync + 'static,
) -> tauri::Result<()> {
    // The shell has no translations yet; the labels hold no `_`, which the
    // menu would read as a shortcut marker.
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, OPEN_FREECAL, "Open FreeCal", true, None::<&str>)?,
            &MenuItem::with_id(app, QUIT, "Quit", true, None::<&str>)?,
        ],
    )?;
    let on_choice = Arc::new(on_choice);
    let on_click = Arc::clone(&on_choice);
    let mut tray = TrayIconBuilder::with_id(TRAY)
        .menu(&menu)
        .tooltip("FreeCal")
        .on_menu_event(move |app, event| {
            let choice = match event.id().as_ref() {
                OPEN_FREECAL => TrayChoice::OpenFreeCal,
                QUIT => TrayChoice::Quit,
                _ => return,
            };
            on_choice(app, choice);
        })
        .on_tray_icon_event(move |icon, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                on_click(icon.app_handle(), TrayChoice::OpenFreeCal);
            }
        });
    // FreeCal's own icon until the status icons of ticket 23.
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
