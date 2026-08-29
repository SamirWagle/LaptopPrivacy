pub mod brightness;
pub mod command_center;
pub mod domain;
pub mod foreground;
pub mod overlay;
pub mod protection;
pub mod storage;

use domain::{AppConfig, AppRule};
use serde::Serialize;
use tauri::{Emitter, Manager};

fn config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(storage::CONFIG_FILE))
        .map_err(|error| format!("could not resolve config directory: {error}"))
}

#[tauri::command]
fn load_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, protection::ProtectionRuntime>,
) -> Result<AppConfig, String> {
    let config = storage::load(&config_path(&app)?)?;
    state.update_config(config.clone())?;
    Ok(config)
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, protection::ProtectionRuntime>,
    config: AppConfig,
) -> Result<(), String> {
    let previous = state.config()?;
    if config.launch_at_login != previous.launch_at_login
        || config.emergency_shortcut != previous.emergency_shortcut
    {
        return Err("Use platform commands to change launch-at-login or shortcuts".into());
    }
    persist_runtime_config(&app, &state, config)
}

pub(crate) fn persist_runtime_config(
    app: &tauri::AppHandle,
    state: &protection::ProtectionRuntime,
    config: AppConfig,
) -> Result<(), String> {
    let path = config_path(app)?;
    let previous = state.config()?;
    storage::save(&path, &config)?;
    if let Err(error) = state.update_config(config) {
        let runtime_rollback = state.update_config(previous.clone());
        let storage_rollback = storage::save(&path, &previous);
        let mut message = format!("could not activate saved config: {error}");
        if let Err(rollback) = runtime_rollback {
            message.push_str(&format!("; runtime rollback failed: {rollback}"));
        }
        if let Err(rollback) = storage_rollback {
            message.push_str(&format!("; storage rollback failed: {rollback}"));
        }
        return Err(message);
    }
    if let Ok(active) = state.config() {
        let _ = app.emit("command-center:config-changed", active);
    }
    Ok(())
}

#[tauri::command]
fn get_hardware_brightness() -> brightness::BrightnessStatus {
    brightness::status()
}

#[tauri::command]
fn preview_hardware_brightness(
    state: tauri::State<'_, protection::ProtectionRuntime>,
    percent: u8,
) -> Result<brightness::BrightnessStatus, String> {
    state.preview_brightness(percent)
}

#[tauri::command]
fn apply_hardware_brightness(
    state: tauri::State<'_, protection::ProtectionRuntime>,
    percent: u8,
) -> Result<brightness::BrightnessStatus, String> {
    state.apply_manual_brightness(percent)
}

#[tauri::command]
fn cancel_hardware_brightness_preview(
    state: tauri::State<'_, protection::ProtectionRuntime>,
) -> Result<(), String> {
    state.restore_brightness()
}

#[tauri::command]
fn get_protection_status(
    state: tauri::State<'_, protection::ProtectionRuntime>,
) -> Result<protection::ProtectionStatus, String> {
    state.status()
}

#[tauri::command]
fn list_running_applications() -> Result<Vec<foreground::ForegroundApplication>, String> {
    foreground::running()
}

#[tauri::command]
fn preview_privacy_overlay(
    state: tauri::State<'_, protection::ProtectionRuntime>,
    visibility_percent: u8,
) -> Result<(), String> {
    state.preview_overlay(visibility_percent)
}

#[tauri::command]
fn cancel_privacy_overlay_preview(
    state: tauri::State<'_, protection::ProtectionRuntime>,
) -> Result<(), String> {
    state.cancel_overlay_preview()
}

#[tauri::command]
fn remove_all_dimming(
    app: tauri::AppHandle,
    state: tauri::State<'_, protection::ProtectionRuntime>,
) -> Result<AppConfig, String> {
    pause_and_persist(&app)?;
    state.config()
}

pub(crate) fn pause_and_persist(app: &tauri::AppHandle) -> Result<AppConfig, String> {
    let state = app.state::<protection::ProtectionRuntime>();
    let cleanup = state.remove_all_dimming();
    let config = state.config()?;
    let save = storage::save(&config_path(app)?, &config);
    let _ = app.emit("command-center:config-changed", config.clone());
    match (cleanup, save) {
        (Ok(()), Ok(())) => {
            state.clear_reported_error();
            Ok(config)
        }
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(cleanup_error), Err(save_error)) => Err(format!(
            "{cleanup_error}; could not persist emergency pause: {save_error}"
        )),
    }
}

#[derive(Serialize)]
struct QuickProtectResult {
    config: AppConfig,
    rule_id: String,
    created: bool,
}

#[tauri::command]
fn protect_current_application(app: tauri::AppHandle) -> Result<QuickProtectResult, String> {
    protect_current_application_inner(&app)
}

pub(crate) fn protect_current_application_inner(
    app: &tauri::AppHandle,
) -> Result<QuickProtectResult, String> {
    let runtime = app.state::<protection::ProtectionRuntime>();
    let foreground = runtime
        .current_application()?
        .ok_or("No foreground application is available to protect")?;
    let mut config = runtime.config()?;
    let candidate_id = format!(
        "quick-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "System clock is unavailable")?
            .as_nanos()
    );
    let (rule_id, created) = protect_application(&mut config, foreground, candidate_id);
    if !created {
        return Ok(QuickProtectResult {
            config,
            rule_id,
            created,
        });
    }
    persist_runtime_config(app, &runtime, config.clone())?;
    runtime.clear_reported_error();
    Ok(QuickProtectResult {
        config,
        rule_id,
        created,
    })
}

fn protect_application(
    config: &mut AppConfig,
    foreground: foreground::ForegroundApplication,
    candidate_id: String,
) -> (String, bool) {
    if let Some(rule) = config
        .app_rules
        .iter()
        .find(|rule| rule.platform_app_id == foreground.platform_app_id)
    {
        return (rule.id.clone(), false);
    }
    config.app_rules.push(AppRule {
        id: candidate_id.clone(),
        platform_app_id: foreground.platform_app_id,
        display_name: foreground.display_name,
        visibility_percent: 35,
        enabled: true,
    });
    (candidate_id, true)
}

#[tauri::command]
fn set_peek_active(
    state: tauri::State<'_, protection::ProtectionRuntime>,
    active: bool,
) -> Result<(), String> {
    state.set_peek_active(active)
}

#[tauri::command]
fn set_launch_at_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, protection::ProtectionRuntime>,
    enabled: bool,
) -> Result<AppConfig, String> {
    let previous_platform = command_center::set_launch_at_login(&app, enabled)?;
    let mut config = state.config()?;
    config.launch_at_login = enabled;
    if let Err(error) = persist_runtime_config(&app, &state, config.clone()) {
        let rollback = command_center::restore_launch_at_login(&app, previous_platform);
        return Err(match rollback {
            Ok(()) => error,
            Err(rollback_error) => {
                format!("{error}; launch-at-login rollback failed: {rollback_error}")
            }
        });
    }
    state.clear_reported_error();
    Ok(config)
}

#[tauri::command]
fn register_shortcuts(
    app: tauri::AppHandle,
    state: tauri::State<'_, protection::ProtectionRuntime>,
    emergency_shortcut: String,
) -> Result<AppConfig, String> {
    let mut config = state.config()?;
    let previous = config.emergency_shortcut.clone();
    command_center::register_emergency(&app, &emergency_shortcut)?;
    config.emergency_shortcut = emergency_shortcut;
    if let Err(error) = persist_runtime_config(&app, &state, config.clone()) {
        let rollback = command_center::register_emergency(&app, &previous);
        return Err(match rollback {
            Ok(()) => error,
            Err(rollback_error) => format!("{error}; shortcut rollback failed: {rollback_error}"),
        });
    }
    state.clear_reported_error();
    Ok(config)
}

pub(crate) fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("command-center:refresh", ());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--autostart")
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(command_center::handle_shortcut)
                .build(),
        )
        .setup(|app| {
            let (config, load_error) =
                match config_path(app.handle()).and_then(|path| storage::load(&path)) {
                    Ok(config) => (config, None),
                    Err(error) => (AppConfig::default(), Some(error)),
                };
            let runtime = protection::ProtectionRuntime::new(config.clone(), app.handle().clone());
            app.manage(runtime.clone());
            let shortcut_error = command_center::install(app, &config)?;
            let autostart_error =
                command_center::sync_launch_at_login(app.handle(), config.launch_at_login).err();
            runtime.start();
            let errors = [load_error, shortcut_error, autostart_error]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                runtime.report_error(errors.join("; "));
            }
            if std::env::args_os().any(|argument| argument == "--autostart") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            get_hardware_brightness,
            preview_hardware_brightness,
            apply_hardware_brightness,
            cancel_hardware_brightness_preview,
            get_protection_status,
            list_running_applications,
            preview_privacy_overlay,
            cancel_privacy_overlay_preview,
            remove_all_dimming,
            protect_current_application,
            set_peek_active,
            set_launch_at_login,
            register_shortcuts
        ])
        .build(tauri::generate_context!())
        .expect("error while building Privacy Aperture");
    app.run(|app_handle, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            app_handle.state::<protection::ProtectionRuntime>().stop();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_protect_reopens_existing_rule_without_duplicate() {
        let application = foreground::ForegroundApplication {
            platform_app_id: "com.example.private".into(),
            display_name: "Private".into(),
            process_id: 7,
        };
        let mut config = AppConfig::default();
        assert_eq!(
            protect_application(&mut config, application.clone(), "first".into()),
            ("first".into(), true)
        );
        assert_eq!(
            protect_application(&mut config, application, "second".into()),
            ("first".into(), false)
        );
        assert_eq!(config.app_rules.len(), 1);
        assert_eq!(config.app_rules[0].visibility_percent, 35);
    }
}
