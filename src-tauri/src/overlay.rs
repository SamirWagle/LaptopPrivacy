use crate::{domain, foreground::WindowBounds};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::{LogicalPosition, LogicalSize, Manager, WindowBuilder};

const LABEL_PREFIX: &str = "privacy-overlay-";

#[derive(Clone)]
pub struct OverlayControl {
    app: tauri::AppHandle,
    state: Arc<Mutex<OverlayState>>,
    generation: Arc<AtomicU64>,
}

#[derive(Default)]
struct OverlayState {
    plan: Option<Vec<DisplayOverlay>>,
    window_count: usize,
    preview_generation: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayOverlay {
    position: LogicalPosition<f64>,
    size: LogicalSize<f64>,
    alpha: u8,
}

impl OverlayControl {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            state: Arc::new(Mutex::new(OverlayState::default())),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn reconcile(
        &self,
        visibility_percent: Option<u8>,
        windows: &[WindowBounds],
    ) -> Result<bool, String> {
        if self
            .state
            .lock()
            .map_err(|_| "overlay state is unavailable")?
            .preview_generation
            .is_some()
        {
            return Ok(true);
        }
        let Some(visibility_percent) = visibility_percent else {
            self.clear_windows()?;
            return Ok(false);
        };
        self.apply(visibility_percent, windows)
    }

    pub fn preview(
        &self,
        visibility_percent: u8,
        windows: Vec<WindowBounds>,
    ) -> Result<(), String> {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.state
            .lock()
            .map_err(|_| "overlay state is unavailable")?
            .preview_generation = Some(generation);
        if let Err(error) = self.apply(visibility_percent, &windows) {
            if let Ok(mut state) = self.state.lock() {
                state.preview_generation = None;
            }
            return Err(error);
        }
        let control = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = control.finish_preview(generation);
        });
        Ok(())
    }

    pub fn cancel_preview(&self) -> Result<(), String> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.state
            .lock()
            .map_err(|_| "overlay state is unavailable")?
            .preview_generation = None;
        self.clear_windows()
    }

    pub fn clear(&self) -> Result<(), String> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.state
            .lock()
            .map_err(|_| "overlay state is unavailable")?
            .preview_generation = None;
        self.clear_windows()
    }

    fn apply(&self, visibility_percent: u8, windows: &[WindowBounds]) -> Result<bool, String> {
        let alpha = alpha_for_visibility(visibility_percent)?;
        if windows.is_empty() {
            self.clear_windows()?;
            return Err("Foreground application has no visible standard window".into());
        }
        let plan: Vec<_> = windows
            .iter()
            .map(|window| DisplayOverlay {
                position: LogicalPosition::new(window.x, window.y),
                size: LogicalSize::new(window.width, window.height),
                alpha,
            })
            .collect();
        let mut state = self
            .state
            .lock()
            .map_err(|_| "overlay state is unavailable")?;
        if state.plan.as_ref() == Some(&plan) {
            return Ok(true);
        }
        for (index, display) in plan.iter().enumerate() {
            let label = format!("{LABEL_PREFIX}{index}");
            let window = if let Some(window) = self.app.get_window(&label) {
                window
            } else {
                WindowBuilder::new(&self.app, &label)
                    .title("Privacy Aperture overlay")
                    .inner_size(1.0, 1.0)
                    .position(0.0, 0.0)
                    .decorations(false)
                    .resizable(false)
                    .maximizable(false)
                    .minimizable(false)
                    .closable(false)
                    .always_on_top(true)
                    .visible_on_all_workspaces(true)
                    .skip_taskbar(true)
                    .shadow(false)
                    .focused(false)
                    .focusable(false)
                    .visible(false)
                    .background_color(tauri::utils::config::Color(0, 0, 0, 255))
                    .build()
                    .map_err(|error| format!("Could not create privacy overlay: {error}"))?
            };
            window
                .set_ignore_cursor_events(true)
                .and_then(|_| window.set_position(display.position))
                .and_then(|_| window.set_size(display.size))
                .and_then(|_| {
                    window.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 255)))
                })
                .map_err(|error| format!("Could not configure privacy overlay: {error}"))?;
            platform::set_opacity(&window, display.alpha)?;
            window
                .show()
                .map_err(|error| format!("Could not configure privacy overlay: {error}"))?;
        }
        for index in plan.len()..state.window_count {
            if let Some(window) = self.app.get_window(&format!("{LABEL_PREFIX}{index}")) {
                let _ = window.hide();
            }
        }
        state.window_count = state.window_count.max(plan.len());
        state.plan = Some(plan);
        Ok(true)
    }

    fn clear_windows(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "overlay state is unavailable")?;
        if state.plan.is_none() {
            return Ok(());
        }
        for index in 0..state.window_count {
            if let Some(window) = self.app.get_window(&format!("{LABEL_PREFIX}{index}")) {
                window
                    .hide()
                    .map_err(|error| format!("Could not hide privacy overlay: {error}"))?;
            }
        }
        state.plan = None;
        Ok(())
    }

    fn finish_preview(&self, generation: u64) -> Result<(), String> {
        if self.generation.load(Ordering::SeqCst) != generation {
            return Ok(());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| "overlay state is unavailable")?;
        if state.preview_generation != Some(generation) {
            return Ok(());
        }
        state.preview_generation = None;
        drop(state);
        self.clear_windows()
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::{c_char, c_void};

    type Selector = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> Selector;
        fn objc_msgSend();
    }

    pub fn set_opacity<R: tauri::Runtime>(
        window: &tauri::Window<R>,
        alpha: u8,
    ) -> Result<(), String> {
        let native = window
            .ns_window()
            .map_err(|error| format!("Could not access macOS overlay window: {error}"))?;
        let function: unsafe extern "C" fn(*mut c_void, Selector, f64) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe {
            function(
                native,
                sel_registerName(c"setAlphaValue:".as_ptr()),
                f64::from(alpha) / 255.0,
            );
        }
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub fn set_opacity<R: tauri::Runtime>(_: &tauri::Window<R>, _: u8) -> Result<(), String> {
        Err("Native privacy overlays are not implemented on this platform yet".into())
    }
}

fn alpha_for_visibility(visibility_percent: u8) -> Result<u8, String> {
    Ok((domain::overlay_opacity(visibility_percent)? * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_maps_to_overlay_alpha() {
        assert_eq!(alpha_for_visibility(100).unwrap(), 0);
        assert_eq!(alpha_for_visibility(30).unwrap(), 179);
        assert!(alpha_for_visibility(9).is_err());
    }
}
