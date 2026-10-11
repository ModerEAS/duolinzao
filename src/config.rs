use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use chrono::NaiveTime;
use toml::Value;

use crate::crypto::decode_local_key;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub super_username: String,
    pub super_password: String,
    pub webauthn_rp_id: String,
    pub webauthn_origin: String,
    pub webauthn_rp_name: String,
    pub email_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub lock_time: NaiveTime,
    pub breakfast_after: NaiveTime,
    pub lunch_after: NaiveTime,
    pub dinner_after: NaiveTime,
    pub mean_threshold: f64,
    pub min_samples: i64,
    pub local_key: [u8; 32],
    pub dish_image_dir: String,
    pub dish_image_max_bytes: u64,
    pub database_url: String,
    pub bind: SocketAddr,
    pub cookie_secure: bool,
    pub trust_proxy: bool,
    pub static_dir: Option<String>,
}

impl AppConfig {
    pub fn load_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let raw =
            fs::read_to_string(path.as_ref()).map_err(|err| format!("无法读取配置: {err}"))?;
        Self::parse(&raw)
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        let value: Value =
            toml::from_str(raw).map_err(|err| format!("配置不是合法 TOML: {err}"))?;
        let mut missing = Vec::new();

        let super_admin = table(&value, "super_admin", &mut missing);
        let webauthn = table(&value, "webauthn", &mut missing);
        let email = table(&value, "email", &mut missing);
        let smtp = table(&value, "smtp", &mut missing);
        let schedule = table(&value, "schedule", &mut missing);
        let score_alert = table(&value, "score_alert", &mut missing);
        let crypto = table(&value, "crypto", &mut missing);
        let dish = table(&value, "dish_image", &mut missing);
        let database = table(&value, "database", &mut missing);
        let server = value.get("server").and_then(Value::as_table).cloned();

        let super_username = req_string(
            super_admin,
            "super_admin.username",
            "username",
            &mut missing,
        );
        let super_password = req_string(
            super_admin,
            "super_admin.password",
            "password",
            &mut missing,
        );
        let rp_id = req_string(webauthn, "webauthn.rp_id", "rp_id", &mut missing);
        let rp_origin = req_string(webauthn, "webauthn.rp_origin", "rp_origin", &mut missing);
        let rp_name = optional_string(webauthn, "rp_name").unwrap_or_else(|| "多邻灶".to_string());
        let email_enabled = optional_bool(email, "enabled").unwrap_or(false);

        let lock_time = req_time(schedule, "schedule.lock_time", "lock_time", &mut missing);
        let breakfast_after = req_time(
            schedule,
            "schedule.breakfast_after",
            "breakfast_after",
            &mut missing,
        );
        let lunch_after = req_time(
            schedule,
            "schedule.lunch_after",
            "lunch_after",
            &mut missing,
        );
        let dinner_after = req_time(
            schedule,
            "schedule.dinner_after",
            "dinner_after",
            &mut missing,
        );
        let mean_threshold = req_f64(
            score_alert,
            "score_alert.mean_threshold",
            "mean_threshold",
            &mut missing,
        );
        let min_samples = req_i64(
            score_alert,
            "score_alert.min_samples",
            "min_samples",
            &mut missing,
        );
        let key_raw = req_string(crypto, "crypto.local_key", "local_key", &mut missing);
        let local_key = match key_raw.as_deref().and_then(decode_local_key) {
            Some(key) => Some(key),
            None => {
                if key_raw.is_some() {
                    missing.push("crypto.local_key".into());
                }
                None
            }
        };
        let dish_image_dir = req_string(dish, "dish_image.dir", "dir", &mut missing);
        let dish_image_max_bytes = req_u64(dish, "dish_image.max_bytes", "max_bytes", &mut missing);
        let database_url = req_string(database, "database.url", "url", &mut missing);

        let mut smtp_host = optional_string(smtp, "host").unwrap_or_default();
        let smtp_port = optional_u16(smtp, "port").unwrap_or(0);
        let smtp_username = optional_string(smtp, "username").unwrap_or_default();
        let smtp_password = optional_string(smtp, "password").unwrap_or_default();
        let mut smtp_from = optional_string(smtp, "from").unwrap_or_default();
        if email_enabled {
            if smtp_host.is_empty() || is_placeholder(&smtp_host) {
                missing.push("smtp.host".into());
                smtp_host.clear();
            }
            if smtp_from.is_empty() || is_placeholder(&smtp_from) {
                missing.push("smtp.from".into());
                smtp_from.clear();
            }
            if smtp_port == 0 {
                missing.push("smtp.port".into());
            }
        }

        if !missing.is_empty() {
            missing.sort();
            missing.dedup();
            return Err(format!(
                "缺少或无效的配置: {}。钟点、阈值、样本量、图片上限和密钥必须由配置给出，代码不设默认值",
                missing.join(", ")
            ));
        }

        let server = server.as_ref();
        let bind = optional_string(server, "bind")
            .unwrap_or_else(|| "0.0.0.0:8080".into())
            .parse()
            .map_err(|_| "server.bind 不是合法地址".to_string())?;
        // 生产默认 true：即使后端容器只听 HTTP（TLS 由 Traefik 等网关终结），
        // 浏览器也只在 HTTPS 下回送 Secure Cookie。本地 HTTP 直连/e2e 才显式配 false。
        let cookie_secure = optional_bool(server, "cookie_secure").unwrap_or(true);
        // 反代可信开关：缺省 false，直接取对端地址；只有确定前面有可信反代
        // （如 Traefik）剥掉一层并设置 X-Forwarded-For 时才配 true。
        let trust_proxy = optional_bool(server, "trust_proxy").unwrap_or(false);
        let static_dir = optional_string(server, "static_dir").filter(|s| !s.is_empty());

        Ok(Self {
            super_username: super_username.unwrap(),
            super_password: super_password.unwrap(),
            webauthn_rp_id: rp_id.unwrap(),
            webauthn_origin: rp_origin.unwrap(),
            webauthn_rp_name: rp_name,
            email_enabled,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            smtp_from,
            lock_time: lock_time.unwrap(),
            breakfast_after: breakfast_after.unwrap(),
            lunch_after: lunch_after.unwrap(),
            dinner_after: dinner_after.unwrap(),
            mean_threshold: mean_threshold.unwrap(),
            min_samples: min_samples.unwrap(),
            local_key: local_key.unwrap(),
            dish_image_dir: dish_image_dir.unwrap(),
            dish_image_max_bytes: dish_image_max_bytes.unwrap(),
            database_url: database_url.unwrap(),
            bind,
            cookie_secure,
            trust_proxy,
            static_dir,
        })
    }
}

fn is_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed == "REPLACE_ME"
        || trimmed == "TODO"
        || trimmed == "CHANGEME"
        || trimmed == "请填写"
}

fn table<'a>(
    root: &'a Value,
    key: &str,
    missing: &mut Vec<String>,
) -> Option<&'a toml::map::Map<String, Value>> {
    match root.get(key).and_then(Value::as_table) {
        Some(table) => Some(table),
        None => {
            missing.push(format!("[{key}]"));
            None
        }
    }
}

fn optional_string(table: Option<&toml::map::Map<String, Value>>, key: &str) -> Option<String> {
    table
        .and_then(|item| item.get(key))
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn optional_bool(table: Option<&toml::map::Map<String, Value>>, key: &str) -> Option<bool> {
    table
        .and_then(|item| item.get(key))
        .and_then(Value::as_bool)
}

fn optional_u16(table: Option<&toml::map::Map<String, Value>>, key: &str) -> Option<u16> {
    let value = table.and_then(|item| item.get(key))?;
    match value {
        Value::Integer(number) if *number > 0 && *number <= u16::MAX as i64 => Some(*number as u16),
        Value::String(text) if !is_placeholder(text) => text.trim().parse().ok(),
        _ => None,
    }
}

fn req_string(
    table: Option<&toml::map::Map<String, Value>>,
    label: &str,
    key: &str,
    missing: &mut Vec<String>,
) -> Option<String> {
    match table.and_then(|item| item.get(key)).and_then(Value::as_str) {
        Some(value) if !is_placeholder(value) => Some(value.trim().to_string()),
        _ => {
            missing.push(label.into());
            None
        }
    }
}

fn req_time(
    table: Option<&toml::map::Map<String, Value>>,
    label: &str,
    key: &str,
    missing: &mut Vec<String>,
) -> Option<NaiveTime> {
    let Some(raw) = req_string(table, label, key, missing) else {
        return None;
    };
    match NaiveTime::parse_from_str(&raw, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(&raw, "%H:%M:%S"))
    {
        Ok(time) => Some(time),
        Err(_) => {
            missing.push(label.into());
            None
        }
    }
}

fn req_f64(
    table: Option<&toml::map::Map<String, Value>>,
    label: &str,
    key: &str,
    missing: &mut Vec<String>,
) -> Option<f64> {
    let Some(table) = table else {
        return None;
    };
    let parsed = match table.get(key) {
        Some(Value::Float(value)) if value.is_finite() => Some(*value),
        Some(Value::Integer(value)) => Some(*value as f64),
        Some(Value::String(value)) if !is_placeholder(value) => value.trim().parse::<f64>().ok(),
        _ => None,
    };
    match parsed.filter(|value| value.is_finite()) {
        Some(value) => Some(value),
        None => {
            missing.push(label.into());
            None
        }
    }
}

fn req_i64(
    table: Option<&toml::map::Map<String, Value>>,
    label: &str,
    key: &str,
    missing: &mut Vec<String>,
) -> Option<i64> {
    let Some(table) = table else {
        return None;
    };
    let parsed = match table.get(key) {
        Some(Value::Integer(value)) if *value >= 0 => Some(*value),
        Some(Value::String(value)) if !is_placeholder(value) => value.trim().parse::<i64>().ok(),
        _ => None,
    };
    match parsed.filter(|value| *value >= 0) {
        Some(value) => Some(value),
        None => {
            missing.push(label.into());
            None
        }
    }
}

fn req_u64(
    table: Option<&toml::map::Map<String, Value>>,
    label: &str,
    key: &str,
    missing: &mut Vec<String>,
) -> Option<u64> {
    let Some(table) = table else {
        return None;
    };
    let parsed = match table.get(key) {
        Some(Value::Integer(value)) if *value > 0 => Some(*value as u64),
        Some(Value::String(value)) if !is_placeholder(value) => value.trim().parse::<u64>().ok(),
        _ => None,
    };
    match parsed.filter(|value| *value > 0) {
        Some(value) => Some(value),
        None => {
            missing.push(label.into());
            None
        }
    }
}

pub fn example_config() -> String {
    format!(
        r#"
[super_admin]
username = "root"
password = "root-password-1"

[webauthn]
rp_id = "localhost"
rp_origin = "http://localhost:8080"
rp_name = "多邻灶"

[email]
enabled = {email}

[smtp]
host = "127.0.0.1"
port = 2525
username = ""
password = ""
from = "noreply@example.com"

[schedule]
lock_time = "{lock}"
breakfast_after = "{breakfast}"
lunch_after = "{lunch}"
dinner_after = "{dinner}"

[score_alert]
mean_threshold = "{threshold}"
min_samples = "{samples}"

[crypto]
local_key = "{key}"

[dish_image]
dir = "{dir}"
max_bytes = "{max_bytes}"

[server]
bind = "127.0.0.1:0"
cookie_secure = false
trust_proxy = false

[database]
url = "postgres://ubuntu@localhost/duolinzao_test"
"#,
        email = "false",
        lock = "15:04",
        breakfast = "08:30",
        lunch = "12:30",
        dinner = "18:30",
        threshold = "0",
        samples = "2",
        key = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=",
        dir = "/tmp/duolinzao-images",
        max_bytes = "1048576",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_file_refuses_to_start() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/config.example.toml");
        let err = AppConfig::load_path(path).expect_err("placeholder config must fail");
        assert!(err.contains("schedule.lock_time"), "{err}");
        assert!(err.contains("score_alert.mean_threshold"), "{err}");
        assert!(err.contains("score_alert.min_samples"), "{err}");
        assert!(err.contains("dish_image.max_bytes"), "{err}");
        assert!(err.contains("crypto.local_key"), "{err}");
    }

    #[test]
    fn email_off_does_not_require_smtp() {
        let mut raw = example_config();
        raw = raw.replace("host = \"127.0.0.1\"", "host = \"\"");
        raw = raw.replace("from = \"noreply@example.com\"", "from = \"\"");
        let config = AppConfig::parse(&raw).expect("email off");
        assert!(!config.email_enabled);
        assert_eq!(config.lock_time.to_string(), "15:04:00");
    }

    #[test]
    fn cookie_secure_defaults_to_true() {
        // 缺省即生产姿态；本地/e2e 在 example_config 里显式 false。
        let raw = example_config().replace("cookie_secure = false", "");
        let config = AppConfig::parse(&raw).expect("缺省应为 true");
        assert!(config.cookie_secure);
        let raw = example_config();
        let config = AppConfig::parse(&raw).expect("显式 false");
        assert!(!config.cookie_secure);
    }

    #[test]
    fn email_on_requires_smtp() {
        let raw = example_config().replace("enabled = false", "enabled = true");
        let mut broken = raw.replace("host = \"127.0.0.1\"", "host = \"\"");
        broken = broken.replace("from = \"noreply@example.com\"", "from = \"\"");
        let err = AppConfig::parse(&broken).expect_err("smtp required");
        assert!(err.contains("smtp.host"), "{err}");
    }
}
