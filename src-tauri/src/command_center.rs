use crate::{domain::AppConfig, protection::ProtectionState};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const PEEK_SHORTCUT: &str = "CommandOrControl+Shift+Space";
const TRAY_ID: &str = "privacy-aperture";
const STATUS_ID: &str = "status";
const PROTECT_ID: &str = "protect-current";
const PEEK_ID: &str = "peek";
const PAUSE_ID: &str = "pause";
const SETTINGS_ID: &str = "settings";
const QUIT_ID: &str = "quit";

#[derive(Default)]
struct RegisteredShortcuts {
    emergency: Option<Shortcut>,
    peek: Option<Shortcut>,
}

pub struct CommandCenter {
    shortcuts: Mutex<RegisteredShortcuts>,
    status_item: MenuItem<tauri::Wry>,
    peek_item: MenuItem<tauri::Wry>,
    pause_item: MenuItem<tauri::Wry>,
    last_state: Mutex<Option<(ProtectionState, bool)>>,
}

pub fn install(app: &mut tauri::App, config: &AppConfig) -> Result<Option<String>, String> {
    let status_item = MenuItem::with_id(app, STATUS_ID, "Watching", false, None::<&str>)
        .map_err(|error| format!("Could not create status menu item: {error}"))?;
    let protect_item =
        MenuItem::with_id(app, PROTECT_ID, "Protect Current App", true, None::<&str>)
            .map_err(|error| format!("Could not create Protect menu item: {error}"))?;
    let peek_item = MenuItem::with_id(app, PEEK_ID, "Temporarily Peek", true, None::<&str>)
        .map_err(|error| format!("Could not create Peek menu item: {error}"))?;
    let pause_item = MenuItem::with_id(app, PAUSE_ID, "Pause All Protection", true, None::<&str>)
        .map_err(|error| format!("Could not create Pause menu item: {error}"))?;
    let settings_item = MenuItem::with_id(app, SETTINGS_ID, "Open Settings", true, None::<&str>)
        .map_err(|error| format!("Could not create Settings menu item: {error}"))?;
    let quit_item = MenuItem::with_id(app, QUIT_ID, "Quit Privacy Aperture", true, None::<&str>)
        .map_err(|error| format!("Could not create Quit menu item: {error}"))?;
    let first_separator = PredefinedMenuItem::separator(app)
        .map_err(|error| format!("Could not create tray separator: {error}"))?;
    let second_separator = PredefinedMenuItem::separator(app)
        .map_err(|error| format!("Could not create tray separator: {error}"))?;
    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &first_separator,
            &protect_item,
            &peek_item,
            &pause_item,
            &second_separator,
            &settings_item,
            &quit_item,
        ],
    )
    .map_err(|error| format!("Could not create tray menu: {error}"))?;

    let center = CommandCenter {
        shortcuts: Mutex::new(RegisteredShortcuts::default()),
        status_item,
        peek_item,
        pause_item,
        last_state: Mutex::new(None),
    };
    app.manage(center);

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Privacy Aperture — Watching")
        .title("○")
        .icon(
            app.default_window_icon()
                .ok_or("Privacy Aperture tray icon is unavailable")?
                .clone(),
        )
        .icon_as_template(true)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .build(app)
        .map_err(|error| format!("Could not create Privacy Aperture menu bar icon: {error}"))?;

    Ok(register_initial_shortcuts(app.handle(), &config.emergency_shortcut).err())
}

pub fn sync_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let current = manager
        .is_enabled()
        .map_err(|error| format!("Could not read launch-at-login state: {error}"))?;
    let Some(enable) = autostart_change(current, enabled) else {
        return Ok(());
    };
    if enable {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| format!("Could not update launch-at-login: {error}"))
}

fn autostart_change(current: bool, requested: bool) -> Option<bool> {
    (current != requested).then_some(requested)
}

pub fn set_launch_at_login(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let previous = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| format!("Could not read launch-at-login state: {error}"))?;
    sync_launch_at_login(app, enabled)?;
    Ok(previous)
}

pub fn restore_launch_at_login(app: &AppHandle, previous: bool) -> Result<(), String> {
    sync_launch_at_login(app, previous)
}

pub fn register_emergency(app: &AppHandle, value: &str) -> Result<(), String> {
    let replacement = parse_shortcut("emergency shortcut", value)?;
    let center = app.state::<CommandCenter>();
    let (current, reserved) = center
        .shortcuts
        .lock()
        .map(|registered| (registered.emergency, registered.peek))
        .map_err(|_| "Shortcut registration state is unavailable".to_string())?;
    replace_registration(
        current,
        replacement,
        reserved,
        |shortcut| {
            app.global_shortcut()
                .register(shortcut)
                .map_err(|error| format!("Shortcut is unavailable: {error}"))
        },
        |shortcut| {
            app.global_shortcut()
                .unregister(shortcut)
                .map_err(|error| format!("Could not release previous shortcut: {error}"))
        },
    )?;
    center
        .shortcuts
        .lock()
        .map_err(|_| "Shortcut registration state is unavailable".to_string())?
        .emergency = Some(replacement);
    Ok(())
}

pub fn update_status(app: &AppHandle, state: ProtectionState) {
    let center = app.state::<CommandCenter>();
    let protection_enabled = app
        .state::<crate::protection::ProtectionRuntime>()
        .config()
        .map(|config| config.enabled)
        .unwrap_or(true);
    let Ok(mut previous) = center.last_state.lock() else {
        return;
    };
    if previous.as_ref() == Some(&(state, protection_enabled)) {
        return;
    }
    *previous = Some((state, protection_enabled));
    drop(previous);
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        update_status_on_main_thread(&app, state, protection_enabled);
    });
}

fn update_status_on_main_thread(app: &AppHandle, state: ProtectionState, protection_enabled: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let center = app.state::<CommandCenter>();
    let (symbol, label, peek_label) = match state {
        ProtectionState::Watching => ("○", "Watching", "Temporarily Peek"),
        ProtectionState::PrivacyActive => ("●", "Protected", "Temporarily Peek"),
        ProtectionState::PeekActive => ("◉", "Peek active", "End Peek"),
        ProtectionState::Paused => ("Ⅱ", "Paused", "Temporarily Peek"),
        ProtectionState::Error => ("!", "Protection error", "Temporarily Peek"),
    };
    let pause_label = if protection_enabled {
        "Pause All Protection"
    } else {
        "Resume Protection"
    };
    let _ = tray.set_title(Some(symbol));
    let _ = tray.set_tooltip(Some(format!("Privacy Aperture — {label}")));
    let _ = center.status_item.set_text(label);
    let _ = center.peek_item.set_text(peek_label);
    let _ = center.peek_item.set_enabled(protection_enabled);
    let _ = center.pause_item.set_text(pause_label);
}

pub fn handle_shortcut(
    app: &AppHandle,
    shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    let center = app.state::<CommandCenter>();
    let Ok(registered) = center.shortcuts.lock() else {
        return;
    };
    let is_emergency = registered
        .emergency
        .is_some_and(|item| item.id() == shortcut.id());
    let is_peek = registered
        .peek
        .is_some_and(|item| item.id() == shortcut.id());
    drop(registered);

    if is_emergency && event.state() == ShortcutState::Pressed {
        if let Err(error) = crate::pause_and_persist(app) {
            app.state::<crate::protection::ProtectionRuntime>()
                .report_error(error);
        }
    } else if is_peek {
        let active = event.state() == ShortcutState::Pressed;
        if let Err(error) = app
            .state::<crate::protection::ProtectionRuntime>()
            .set_peek_active(active)
        {
            app.state::<crate::protection::ProtectionRuntime>()
                .report_error(error);
        }
    }
}

fn register_initial_shortcuts(app: &AppHandle, emergency: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(error) = register_emergency(app, emergency) {
        errors.push(error);
    }
    let peek = parse_shortcut("Peek shortcut", PEEK_SHORTCUT)?;
    let center = app.state::<CommandCenter>();
    let emergency = center
        .shortcuts
        .lock()
        .map_err(|_| "Shortcut registration state is unavailable".to_string())?
        .emergency;
    if emergency.is_some_and(|item| item.id() == peek.id()) {
        errors.push("Peek shortcut conflicts with emergency shortcut".into());
    } else {
        match app.global_shortcut().register(peek) {
            Ok(()) => {
                center
                    .shortcuts
                    .lock()
                    .map_err(|_| "Shortcut registration state is unavailable".to_string())?
                    .peek = Some(peek);
            }
            Err(error) => errors.push(format!("Peek shortcut is unavailable: {error}")),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn parse_shortcut(label: &str, value: &str) -> Result<Shortcut, String> {
    value
        .parse()
        .map_err(|error| format!("Invalid {label}: {error}"))
}

fn replace_registration<T: Copy + Eq>(
    current: Option<T>,
    replacement: T,
    reserved: Option<T>,
    mut register: impl FnMut(T) -> Result<(), String>,
    mut unregister: impl FnMut(T) -> Result<(), String>,
) -> Result<(), String> {
    if current == Some(replacement) {
        return Ok(());
    }
    if reserved == Some(replacement) {
        return Err("Shortcut conflicts with Peek shortcut".into());
    }
    register(replacement)?;
    if let Some(previous) = current {
        if let Err(error) = unregister(previous) {
            let rollback = unregister(replacement);
            return Err(match rollback {
                Ok(()) => error,
                Err(rollback_error) => {
                    format!("{error}; shortcut rollback failed: {rollback_error}")
                }
            });
        }
    }
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    let result = match id {
        PROTECT_ID => crate::protect_current_application_inner(app).map(|result| {
            crate::show_settings(app);
            let _ = tauri::Emitter::emit(app, "command-center:edit-application", result.rule_id);
        }),
        PEEK_ID => {
            let runtime = app.state::<crate::protection::ProtectionRuntime>();
            runtime.set_peek_active(!runtime.peek_active())
        }
        PAUSE_ID => {
            let runtime = app.state::<crate::protection::ProtectionRuntime>();
            match runtime.config() {
                Ok(config) if config.enabled => crate::pause_and_persist(app).map(|_| ()),
                Ok(mut config) => {
                    config.enabled = true;
                    crate::persist_runtime_config(app, &runtime, config)
                }
                Err(error) => Err(error),
            }
        }
        SETTINGS_ID => {
            crate::show_settings(app);
            Ok(())
        }
        QUIT_ID => {
            app.state::<crate::protection::ProtectionRuntime>().stop();
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        app.state::<crate::protection::ProtectionRuntime>()
            .report_error(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn default_shortcuts_parse_and_do_not_conflict() {
        let emergency = parse_shortcut("emergency shortcut", "CommandOrControl+Shift+0").unwrap();
        let peek = parse_shortcut("Peek shortcut", PEEK_SHORTCUT).unwrap();
        assert_ne!(emergency.id(), peek.id());
    }

    #[test]
    fn autostart_sync_changes_only_mismatched_state() {
        assert_eq!(autostart_change(false, false), None);
        assert_eq!(autostart_change(true, true), None);
        assert_eq!(autostart_change(false, true), Some(true));
        assert_eq!(autostart_change(true, false), Some(false));
    }

    #[test]
    fn shortcut_conflict_keeps_previous_registration() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let register_calls = calls.clone();
        let unregister_calls = calls.clone();
        let result = replace_registration(
            Some(1),
            2,
            Some(2),
            move |value| {
                register_calls.borrow_mut().push(("register", value));
                Ok(())
            },
            move |value| {
                unregister_calls.borrow_mut().push(("unregister", value));
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn failed_old_unregistration_removes_new_shortcut() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let register_calls = calls.clone();
        let unregister_calls = calls.clone();
        let result = replace_registration(
            Some(1),
            2,
            None,
            move |value| {
                register_calls.borrow_mut().push(("register", value));
                Ok(())
            },
            move |value| {
                unregister_calls.borrow_mut().push(("unregister", value));
                if value == 1 {
                    Err("old registration busy".into())
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(
            calls.borrow().as_slice(),
            &[("register", 2), ("unregister", 1), ("unregister", 2)]
        );
    }
}
