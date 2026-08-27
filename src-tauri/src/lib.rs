pub mod brightness;
pub mod domain;
pub mod storage;

use domain::AppConfig;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::Manager;

#[derive(Clone, Default)]
struct HardwareBrightnessControl(Arc<PreviewInner>);

#[derive(Default)]
struct PreviewInner {
    generation: AtomicU64,
    original: Mutex<Option<brightness::Snapshot>>,
}

impl HardwareBrightnessControl {
    fn start(&self, percent: u8) -> Result<brightness::BrightnessStatus, String> {
        let (status, generation) = self.apply(percent)?;
        let preview = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            if preview.0.generation.load(Ordering::SeqCst) == generation {
                let _ = preview.restore_current();
            }
        });
        Ok(status)
    }

    fn start_until_cancelled(&self, percent: u8) -> Result<brightness::BrightnessStatus, String> {
        self.apply(percent).map(|(status, _)| status)
    }

    fn apply(&self, percent: u8) -> Result<(brightness::BrightnessStatus, u64), String> {
        self.cancel()?;
        let generation = self.0.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (status, snapshot) = brightness::apply(percent)?;
        *self
            .0
            .original
            .lock()
            .map_err(|_| "brightness preview state is unavailable")? = Some(snapshot);
        Ok((status, generation))
    }

    fn cancel(&self) -> Result<(), String> {
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        self.restore_current()
    }

    fn restore_current(&self) -> Result<(), String> {
        let snapshot = self
            .0
            .original
            .lock()
            .map_err(|_| "brightness preview state is unavailable")?
            .take();
        snapshot.as_ref().map_or(Ok(()), brightness::restore)
    }
}

fn config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(storage::CONFIG_FILE))
        .map_err(|error| format!("could not resolve config directory: {error}"))
}

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    storage::load(&config_path(&app)?)
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    storage::save(&config_path(&app)?, &config)
}

#[tauri::command]
fn get_hardware_brightness() -> brightness::BrightnessStatus {
    brightness::status()
}

#[tauri::command]
fn preview_hardware_brightness(
    state: tauri::State<'_, HardwareBrightnessControl>,
    percent: u8,
) -> Result<brightness::BrightnessStatus, String> {
    state.start(percent)
}

#[tauri::command]
fn apply_hardware_brightness(
    state: tauri::State<'_, HardwareBrightnessControl>,
    percent: u8,
) -> Result<brightness::BrightnessStatus, String> {
    state.start_until_cancelled(percent)
}

#[tauri::command]
fn cancel_hardware_brightness_preview(
    state: tauri::State<'_, HardwareBrightnessControl>,
) -> Result<(), String> {
    state.cancel()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(HardwareBrightnessControl::default())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            get_hardware_brightness,
            preview_hardware_brightness,
            apply_hardware_brightness,
            cancel_hardware_brightness_preview
        ])
        .build(tauri::generate_context!())
        .expect("error while building Privacy Aperture");
    app.run(|app_handle, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            let _ = app_handle.state::<HardwareBrightnessControl>().cancel();
        }
    });
}
