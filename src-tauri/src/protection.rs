use crate::{
    brightness::BrightnessControl,
    domain::{self, AppConfig, ForegroundContext},
    foreground::{self, ForegroundApplication},
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
    pub fn new(config: AppConfig) -> Self {
        Self(Arc::new(RuntimeInner {
            config: RwLock::new(config),
            status: Mutex::new(ProtectionStatus::default()),
            brightness: BrightnessControl::default(),
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

    pub fn stop(&self) {
        self.0.stopped.store(true, Ordering::Relaxed);
        if let Ok(mut worker) = self.0.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
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
        let result = match target.as_ref().and_then(|target| {
            target
                .hardware_percent
                .map(|percent| (target.rule_id.clone(), percent))
        }) {
            Some((rule_id, percent)) => self.0.brightness.reconcile_automatic(rule_id, percent),
            None => self.0.brightness.clear_automatic(),
        };
        let (hardware_active, message) = match result {
            Ok(active) => (
                active,
                if target.is_some() {
                    if active {
                        "Protected application matched; hardware brightness reduced".into()
                    } else {
                        "Protected application matched".into()
                    }
                } else {
                    "No protected application matched".into()
                },
            ),
            Err(error) => (false, error),
        };
        if let Ok(mut status) = self.0.status.lock() {
            *status = ProtectionStatus {
                foreground_supported: foreground::supported(),
                foreground_app: foreground,
                matched_rule_id: target.as_ref().map(|target| target.rule_id.clone()),
                matched_visibility_percent: target.map(|target| target.visibility_percent),
                hardware_active,
                message,
            };
        }
    }

    fn set_error(&self, message: String) {
        let _ = self.0.brightness.clear_automatic();
        if let Ok(mut status) = self.0.status.lock() {
            status.hardware_active = false;
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
        visibility_percent: matched.visibility_percent,
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
        };
        assert_eq!(
            protection_target(&config, &app),
            Some(ProtectionTarget {
                rule_id: "private-mail".into(),
                visibility_percent: 30,
                hardware_percent: Some(42),
            })
        );
        assert_eq!(
            protection_target(
                &AppConfig {
                    maximum_privacy: true,
                    ..config
                },
                &app,
            )
            .unwrap()
            .hardware_percent,
            Some(10)
        );
    }
}
