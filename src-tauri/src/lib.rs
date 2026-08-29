pub mod brightness;
pub mod domain;
pub mod foreground;
pub mod overlay;
pub mod protection;
pub mod storage;

use domain::AppConfig;
use tauri::Manager;

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
    let path = config_path(&app)?;
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
    let cleanup = state.remove_all_dimming();
    let config = state.config()?;
    let save = storage::save(&config_path(&app)?, &config);
    match (cleanup, save) {
        (Ok(()), Ok(())) => Ok(config),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(cleanup_error), Err(save_error)) => Err(format!(
            "{cleanup_error}; could not persist emergency pause: {save_error}"
        )),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let runtime =
                protection::ProtectionRuntime::new(AppConfig::default(), app.handle().clone());
            runtime.start();
            app.manage(runtime);
            Ok(())
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
            remove_all_dimming
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
