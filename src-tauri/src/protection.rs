use crate::{
    brightness::BrightnessControl,
    domain::{self, AppConfig, ForegroundContext},
    foreground::{self, ForegroundApplication},
    overlay::OverlayControl,
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(150);

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProtectionStatus {
    pub foreground_supported: bool,
    pub foreground_app: Option<ForegroundApplication>,
    pub matched_rule_id: Option<String>,
    pub matched_visibility_percent: Option<u8>,
    pub hardware_active: bool,
    pub overlay_active: bool,
    pub message: String,
}

impl Default for ProtectionStatus {
    fn default() -> Self {
        Self {
            foreground_supported: foreground::supported(),
            foreground_app: None,
            matched_rule_id: None,
            matched_visibility_percent: None,
            hardware_active: false,
            overlay_active: false,
            message: if foreground::supported() {
                "Watching current foreground application locally".into()
            } else {
                "Foreground application automation is not available on this platform yet".into()
            },
        }
    }
}

#[derive(Clone)]
pub struct ProtectionRuntime(Arc<RuntimeInner>);

struct RuntimeInner {
    config: RwLock<AppConfig>,
    status: Mutex<ProtectionStatus>,
    brightness: BrightnessControl,
    overlay: OverlayControl,
    stopped: AtomicBool,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

#[derive(Debug, PartialEq)]
struct ProtectionTarget {
    rule_id: String,
    visibility_percent: u8,
    hardware_percent: Option<u8>,
}

impl ProtectionRuntime {
    pub fn new(config: AppConfig, app: tauri::AppHandle) -> Self {
        Self(Arc::new(RuntimeInner {
            config: RwLock::new(config),
            status: Mutex::new(ProtectionStatus::default()),
            brightness: BrightnessControl::default(),
            overlay: OverlayControl::new(app),
            stopped: AtomicBool::new(false),
            worker: Mutex::new(None),
        }))
    }

    pub fn start(&self) {
        let runtime = self.clone();
        let worker = std::thread::spawn(move || {
            while !runtime.0.stopped.load(Ordering::Relaxed) {
                runtime.refresh();
                std::thread::sleep(POLL_INTERVAL);
            }
        });
        if let Ok(mut slot) = self.0.worker.lock() {
            *slot = Some(worker);
        }
    }

    pub fn config(&self) -> Result<AppConfig, String> {
        self.0
            .config
            .read()
            .map(|config| config.clone())
            .map_err(|_| "protection config is unavailable".into())
    }

    pub fn update_config(&self, config: AppConfig) -> Result<(), String> {
        let paused = !config.enabled;
        *self
            .0
            .config
            .write()
            .map_err(|_| "protection config is unavailable")? = config;
        if paused {
            self.0.brightness.cancel()?;
        }
        self.refresh();
        Ok(())
    }

    pub fn status(&self) -> Result<ProtectionStatus, String> {
        self.0
            .status
            .lock()
            .map(|status| status.clone())
            .map_err(|_| "protection status is unavailable".into())
    }

    pub fn preview_brightness(
        &self,
        percent: u8,
    ) -> Result<crate::brightness::BrightnessStatus, String> {
        self.0.brightness.preview(percent)
    }

    pub fn apply_manual_brightness(
        &self,
        percent: u8,
    ) -> Result<crate::brightness::BrightnessStatus, String> {
        self.0.brightness.apply_manual(percent)
    }

    pub fn restore_brightness(&self) -> Result<(), String> {
        self.0.brightness.cancel()
    }

    pub fn preview_overlay(&self, visibility_percent: u8) -> Result<(), String> {
        let app = foreground::current()?
            .ok_or_else(|| "No foreground application available for preview".to_string())?;
        let windows = foreground::window_bounds(app.process_id)?;
        self.0.overlay.preview(visibility_percent, windows)
    }

    pub fn cancel_overlay_preview(&self) -> Result<(), String> {
        self.0.overlay.cancel_preview()
    }

    pub fn remove_all_dimming(&self) -> Result<(), String> {
        self.0.overlay.clear()?;
        self.0.brightness.cancel()
    }

    pub fn stop(&self) {
        self.0.stopped.store(true, Ordering::Relaxed);
        if let Ok(mut worker) = self.0.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
        let _ = self.0.overlay.clear();
        let _ = self.0.brightness.cancel();
    }

    fn refresh(&self) {
        let foreground = match foreground::current() {
            Ok(value) => value,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let config = match self.config() {
            Ok(config) => config,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let target = foreground
            .as_ref()
            .and_then(|app| protection_target(&config, app));
        let brightness_result = match target.as_ref().and_then(|target| {
            target
                .hardware_percent
                .map(|percent| (target.rule_id.clone(), percent))
        }) {
            Some((rule_id, percent)) => self.0.brightness.reconcile_automatic(rule_id, percent),
            None => self.0.brightness.clear_automatic(),
        };
        let overlay_result = match (target.as_ref(), foreground.as_ref()) {
            (Some(target), Some(app)) => {
                foreground::window_bounds(app.process_id).and_then(|windows| {
                    self.0
                        .overlay
                        .reconcile(Some(target.visibility_percent), &windows)
                })
            }
            _ => self.0.overlay.reconcile(None, &[]),
        };
        let (hardware_active, overlay_active, message) = match (brightness_result, overlay_result) {
            (Ok(hardware_active), Ok(overlay_active)) => {
                let message = if target.is_some() {
                    match (hardware_active, overlay_active) {
                        (true, true) => {
                            "Protected application matched; overlay and hardware brightness active"
                        }
                        (true, false) => {
                            "Protected application matched; hardware brightness active"
                        }
                        (false, true) => "Protected application matched; overlay active",
                        (false, false) => "Protected application matched",
                    }
                } else {
                    "No protected application matched"
                };
                (hardware_active, overlay_active, message.into())
            }
            (Err(error), Ok(overlay_active)) => (false, overlay_active, error),
            (Ok(hardware_active), Err(error)) => (hardware_active, false, error),
            (Err(brightness_error), Err(overlay_error)) => {
                (false, false, format!("{brightness_error}; {overlay_error}"))
            }
        };
        if let Ok(mut status) = self.0.status.lock() {
            *status = ProtectionStatus {
                foreground_supported: foreground::supported(),
                foreground_app: foreground,
                matched_rule_id: target.as_ref().map(|target| target.rule_id.clone()),
                matched_visibility_percent: target.map(|target| target.visibility_percent),
                hardware_active,
                overlay_active,
                message,
            };
        }
    }

    fn set_error(&self, message: String) {
        let _ = self.0.brightness.clear_automatic();
        let _ = self.0.overlay.clear();
        if let Ok(mut status) = self.0.status.lock() {
            status.hardware_active = false;
            status.overlay_active = false;
            status.message = message;
        }
    }
}

fn protection_target(
    config: &AppConfig,
    foreground: &ForegroundApplication,
) -> Option<ProtectionTarget> {
    let matched = domain::evaluate(
        config,
        &ForegroundContext {
            platform_app_id: &foreground.platform_app_id,
            browser_hostname: None,
        },
    )?;
    Some(ProtectionTarget {
        rule_id: matched.rule_id.to_string(),
        visibility_percent: if config.maximum_privacy {
            10
        } else {
            matched.visibility_percent
        },
        hardware_percent: config
            .hardware_brightness_enabled
            .then_some(if config.maximum_privacy {
                10
            } else {
                config.privacy_brightness_percent
            }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AppRule;

    #[test]
    fn matching_app_selects_configured_hardware_level() {
        let config = AppConfig {
            hardware_brightness_enabled: true,
            privacy_brightness_percent: 42,
            app_rules: vec![AppRule {
                id: "private-mail".into(),
                platform_app_id: "com.example.mail".into(),
                display_name: "Mail".into(),
                visibility_percent: 30,
                enabled: true,
            }],
            ..AppConfig::default()
        };
        let app = ForegroundApplication {
            platform_app_id: "com.example.mail".into(),
            display_name: "Mail".into(),
            process_id: 42,
        };
        assert_eq!(
            protection_target(&config, &app),
            Some(ProtectionTarget {
                rule_id: "private-mail".into(),
                visibility_percent: 30,
                hardware_percent: Some(42),
            })
        );
        let maximum = protection_target(
            &AppConfig {
                maximum_privacy: true,
                ..config
            },
            &app,
        )
        .unwrap();
        assert_eq!(maximum.hardware_percent, Some(10));
        assert_eq!(maximum.visibility_percent, 10);
    }
}
