use crate::domain::AppConfig;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

pub const CONFIG_FILE: &str = "config.json";

pub fn load(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let bytes = fs::read(path).map_err(|error| format!("could not read config: {error}"))?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not parse config: {error}"))?;
    let version = value
        .get("config_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or("config version is missing or invalid")?;
    if version == 1 {
        let object = value
            .as_object_mut()
            .ok_or("config must be a JSON object")?;
        object.insert("config_version".into(), 2.into());
        object.insert("hardware_brightness_enabled".into(), false.into());
        object.insert("privacy_brightness_percent".into(), 35.into());
        object.insert("maximum_privacy".into(), false.into());
    } else if version != u64::from(crate::domain::CONFIG_VERSION) {
        return Err(format!(
            "unsupported config version {version}; expected {}",
            crate::domain::CONFIG_VERSION
        ));
    }
    let config: AppConfig = serde_json::from_value(value)
        .map_err(|error| format!("could not parse config: {error}"))?;
    config.validate()?;
    Ok(config)
}

pub fn save(path: &Path, config: &AppConfig) -> Result<(), String> {
    config.validate()?;
    let parent = path.parent().ok_or("config path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create config directory: {error}"))?;
    let temporary = temporary_path(path);
    let bytes = serde_json::to_vec_pretty(config)
        .map_err(|error| format!("could not encode config: {error}"))?;
    let mut file = File::create(&temporary)
        .map_err(|error| format!("could not create temporary config: {error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("could not write temporary config: {error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("could not replace config: {error}"))?;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension("json.tmp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn test_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "privacy-aperture-{}-{}/config.json",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn atomically_saves_and_reloads_config() {
        let path = test_path();
        let config = AppConfig {
            launch_at_login: true,
            ..AppConfig::default()
        };
        save(&path, &config).unwrap();
        assert_eq!(load(&path).unwrap(), config);
        assert!(!temporary_path(&path).exists());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn rejects_unknown_config_version() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            br#"{"config_version":99,"enabled":true,"launch_at_login":false,"emergency_shortcut":"Ctrl+Shift+0","app_rules":[],"site_rules":[]}"#,
        )
        .unwrap();
        assert!(load(&path)
            .unwrap_err()
            .contains("unsupported config version"));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn migrates_v1_without_losing_rules() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            br#"{"config_version":1,"enabled":true,"launch_at_login":false,"emergency_shortcut":"Ctrl+Shift+0","app_rules":[{"id":"mail","platform_app_id":"com.example.mail","display_name":"Mail","visibility_percent":40,"enabled":true}],"site_rules":[]}"#,
        )
        .unwrap();
        let config = load(&path).unwrap();
        assert_eq!(config.config_version, 2);
        assert_eq!(config.app_rules[0].platform_app_id, "com.example.mail");
        assert!(!config.hardware_brightness_enabled);
        assert_eq!(config.privacy_brightness_percent, 35);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
