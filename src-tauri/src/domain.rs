use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const CONFIG_VERSION: u32 = 2;
pub const MESSAGE_VERSION: u32 = 1;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppRule {
    pub id: String,
    pub platform_app_id: String,
    pub display_name: String,
    pub visibility_percent: u8,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SiteRule {
    pub id: String,
    pub hostname: String,
    pub include_subdomains: bool,
    pub visibility_percent: u8,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub config_version: u32,
    pub enabled: bool,
    pub launch_at_login: bool,
    pub emergency_shortcut: String,
    pub hardware_brightness_enabled: bool,
    pub privacy_brightness_percent: u8,
    pub maximum_privacy: bool,
    pub app_rules: Vec<AppRule>,
    pub site_rules: Vec<SiteRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            enabled: true,
            launch_at_login: false,
            emergency_shortcut: "CommandOrControl+Shift+0".into(),
            hardware_brightness_enabled: false,
            privacy_brightness_percent: 35,
            maximum_privacy: false,
            app_rules: Vec::new(),
            site_rules: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.config_version != CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {}; expected {CONFIG_VERSION}",
                self.config_version
            ));
        }
        if self.emergency_shortcut.trim().is_empty() || self.emergency_shortcut.len() > 80 {
            return Err("emergency shortcut must contain 1..80 characters".into());
        }
        validate_visibility(self.privacy_brightness_percent)?;
        let mut application_ids = HashSet::new();
        for rule in &self.app_rules {
            validate_text("application rule id", &rule.id, 128)?;
            validate_text("platform application id", &rule.platform_app_id, 512)?;
            validate_text("application display name", &rule.display_name, 160)?;
            validate_visibility(rule.visibility_percent)?;
            if !application_ids.insert(rule.platform_app_id.as_str()) {
                return Err(format!(
                    "duplicate platform application id: {}",
                    rule.platform_app_id
                ));
            }
        }
        for rule in &self.site_rules {
            validate_text("site rule id", &rule.id, 128)?;
            validate_visibility(rule.visibility_percent)?;
            validate_hostname(&rule.hostname)?;
            if rule.hostname != rule.hostname.to_ascii_lowercase() {
                return Err("hostname must be lowercase".into());
            }
        }
        Ok(())
    }
}

fn validate_text(label: &str, value: &str, max: usize) -> Result<(), String> {
    let length = value.trim().len();
    if length == 0 || length > max {
        Err(format!("{label} must contain 1..{max} characters"))
    } else {
        Ok(())
    }
}

pub fn validate_visibility(value: u8) -> Result<(), String> {
    if (10..=100).contains(&value) {
        Ok(())
    } else {
        Err("visibility must be between 10 and 100".into())
    }
}

pub fn validate_hostname(hostname: &str) -> Result<(), String> {
    if hostname.is_empty()
        || hostname.len() > 253
        || hostname.starts_with('.')
        || hostname.ends_with('.')
    {
        return Err("hostname must contain 1..253 characters without edge dots".into());
    }
    if hostname.split('.').any(|label| {
        label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }) {
        return Err("hostname contains an invalid label".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeMessage {
    pub version: u32,
    pub kind: String,
    pub browser_instance: String,
    pub sequence: u64,
    pub browser_active: bool,
    pub hostname: Option<String>,
}

#[derive(Default)]
pub struct NativeMessageValidator {
    last_sequences: HashMap<String, u64>,
}

impl NativeMessageValidator {
    pub fn parse(&mut self, bytes: &[u8]) -> Result<NativeMessage, String> {
        if bytes.len() > MAX_MESSAGE_BYTES {
            return Err("native message exceeds 64 KiB".into());
        }
        let message: NativeMessage =
            serde_json::from_slice(bytes).map_err(|_| "malformed native message".to_string())?;
        if message.version != MESSAGE_VERSION || message.kind != "active_context" {
            return Err("unsupported native message".into());
        }
        validate_text("browser instance", &message.browser_instance, 128)?;
        match (&message.hostname, message.browser_active) {
            (Some(hostname), true) => {
                validate_hostname(hostname)?;
                if hostname != &hostname.to_ascii_lowercase() {
                    return Err("hostname must be lowercase".into());
                }
            }
            (None, false) => {}
            _ => return Err("hostname must exist only while browser is active".into()),
        }
        if self
            .last_sequences
            .get(&message.browser_instance)
            .is_some_and(|last| message.sequence <= *last)
        {
            return Err("stale native message sequence".into());
        }
        self.last_sequences
            .insert(message.browser_instance.clone(), message.sequence);
        Ok(message)
    }

    pub fn disconnect(&mut self, browser_instance: &str) {
        self.last_sequences.remove(browser_instance);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForegroundContext<'a> {
    pub platform_app_id: &'a str,
    pub browser_hostname: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchKind {
    Application,
    Website,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProtectionMatch<'a> {
    pub kind: MatchKind,
    pub rule_id: &'a str,
    pub visibility_percent: u8,
}

pub fn evaluate<'a>(
    config: &'a AppConfig,
    context: &ForegroundContext<'_>,
) -> Option<ProtectionMatch<'a>> {
    if !config.enabled {
        return None;
    }
    if let Some(hostname) = context.browser_hostname {
        if let Some(rule) = config
            .site_rules
            .iter()
            .filter(|rule| rule.enabled && hostname_matches(hostname, rule))
            .max_by_key(|rule| rule.hostname.len())
        {
            return Some(ProtectionMatch {
                kind: MatchKind::Website,
                rule_id: &rule.id,
                visibility_percent: rule.visibility_percent,
            });
        }
    }
    config
        .app_rules
        .iter()
        .find(|rule| rule.enabled && rule.platform_app_id == context.platform_app_id)
        .map(|rule| ProtectionMatch {
            kind: MatchKind::Application,
            rule_id: &rule.id,
            visibility_percent: rule.visibility_percent,
        })
}

pub fn hostname_matches(hostname: &str, rule: &SiteRule) -> bool {
    hostname == rule.hostname
        || (rule.include_subdomains
            && hostname
                .strip_suffix(&rule.hostname)
                .is_some_and(|prefix| prefix.ends_with('.') && prefix.len() > 1))
}

pub fn overlay_opacity(visibility_percent: u8) -> Result<f64, String> {
    validate_visibility(visibility_percent)?;
    Ok(f64::from(100 - visibility_percent) / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AppConfig {
        AppConfig {
            app_rules: vec![AppRule {
                id: "bank-app".into(),
                platform_app_id: "com.example.bank".into(),
                display_name: "Bank".into(),
                visibility_percent: 45,
                enabled: true,
            }],
            site_rules: vec![SiteRule {
                id: "bank-site".into(),
                hostname: "bank.example".into(),
                include_subdomains: true,
                visibility_percent: 30,
                enabled: true,
            }],
            ..AppConfig::default()
        }
    }

    #[test]
    fn exact_and_subdomain_matching_respects_boundary() {
        let rule = &config().site_rules[0];
        assert!(hostname_matches("bank.example", rule));
        assert!(hostname_matches("secure.bank.example", rule));
        assert!(!hostname_matches("fakebank.example", rule));
        assert!(!hostname_matches("bank.example.attacker.test", rule));
    }

    #[test]
    fn site_overrides_app_and_pause_overrides_everything() {
        let mut config = config();
        let context = ForegroundContext {
            platform_app_id: "com.example.bank",
            browser_hostname: Some("secure.bank.example"),
        };
        let matched = evaluate(&config, &context).expect("rule should match");
        assert_eq!(matched.kind, MatchKind::Website);
        assert_eq!(matched.visibility_percent, 30);
        config.enabled = false;
        assert_eq!(evaluate(&config, &context), None);
    }

    #[test]
    fn most_specific_hostname_rule_wins() {
        let mut config = config();
        config.site_rules.insert(
            0,
            SiteRule {
                id: "parent-site".into(),
                hostname: "example".into(),
                include_subdomains: true,
                visibility_percent: 70,
                enabled: true,
            },
        );
        let matched = evaluate(
            &config,
            &ForegroundContext {
                platform_app_id: "com.example.browser",
                browser_hostname: Some("bank.example"),
            },
        )
        .expect("specific site rule should match");
        assert_eq!(matched.rule_id, "bank-site");
        assert_eq!(matched.visibility_percent, 30);
    }

    #[test]
    fn config_rejects_duplicate_application_matchers() {
        let mut config = config();
        config.app_rules.push(AppRule {
            id: "bank-app-copy".into(),
            platform_app_id: "com.example.bank".into(),
            display_name: "Bank copy".into(),
            visibility_percent: 20,
            enabled: true,
        });
        assert_eq!(
            config.validate().unwrap_err(),
            "duplicate platform application id: com.example.bank"
        );
    }

    #[test]
    fn browser_context_does_not_apply_after_focus_loss() {
        let config = config();
        let context = ForegroundContext {
            platform_app_id: "com.example.notes",
            browser_hostname: None,
        };
        assert_eq!(evaluate(&config, &context), None);
    }

    #[test]
    fn visibility_maps_to_black_opacity() {
        assert_eq!(overlay_opacity(30).unwrap(), 0.7);
        assert!(overlay_opacity(9).is_err());
        assert!(overlay_opacity(101).is_err());
    }

    #[test]
    fn native_messages_reject_malformed_stale_and_unknown_data() {
        let mut validator = NativeMessageValidator::default();
        let valid = br#"{"version":1,"kind":"active_context","browser_instance":"session-a","sequence":42,"browser_active":true,"hostname":"web.example"}"#;
        assert!(validator.parse(valid).is_ok());
        assert_eq!(
            validator.parse(valid).unwrap_err(),
            "stale native message sequence"
        );
        assert!(validator.parse(b"not json").is_err());
        assert!(validator.parse(br#"{"version":2}"#).is_err());
        assert!(validator.parse(br#"{"version":1,"kind":"active_context","browser_instance":"b","sequence":1,"browser_active":true,"hostname":"web.example","url":"https://web.example/private"}"#).is_err());
    }
}
