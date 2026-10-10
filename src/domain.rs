use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Asia::Shanghai;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub const SLOTS: [&str; 3] = ["breakfast", "lunch", "dinner"];
pub const KINDS: [&str; 3] = ["菜", "饭", "汤"];

pub fn valid_slot(slot: &str) -> bool {
    SLOTS.contains(&slot)
}

pub fn valid_kind(kind: &str) -> bool {
    KINDS.contains(&kind)
}

pub fn empty_groups() -> Map<String, Value> {
    let mut groups = Map::new();
    for kind in KINDS {
        groups.insert((*kind).to_string(), json!([]));
    }
    groups
}

pub fn push_group(groups: &mut Map<String, Value>, kind: &str, item: Value) {
    if let Some(Value::Array(items)) = groups.get_mut(kind) {
        items.push(item);
    }
}

pub fn shanghai_at(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    Shanghai
        .from_local_datetime(&date.and_time(time))
        .single()
        .expect("上海时间唯一")
        .with_timezone(&Utc)
}

pub fn shanghai_date(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Shanghai).date_naive()
}

pub fn lock_deadline(meal_date: NaiveDate, lock_time: NaiveTime) -> DateTime<Utc> {
    let previous = meal_date.pred_opt().expect("日期");
    shanghai_at(previous, lock_time)
}

pub fn meal_is_locked(meal_date: NaiveDate, lock_time: NaiveTime, now: DateTime<Utc>) -> bool {
    now >= lock_deadline(meal_date, lock_time)
}

pub fn score_window_open(meal_date: NaiveDate, after: NaiveTime, now: DateTime<Utc>) -> bool {
    let start = shanghai_at(meal_date, after);
    let end = start + Duration::days(7);
    now >= start && now < end
}

pub fn is_work_day(date: NaiveDate, anchor: NaiveDate) -> bool {
    (date - anchor).num_days().rem_euclid(6) < 2
}

pub fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

pub fn next_slot(date: NaiveDate, slot: &str) -> (NaiveDate, &'static str) {
    match slot {
        "breakfast" => (date, "lunch"),
        "lunch" => (date, "dinner"),
        _ => (date + Duration::days(1), "breakfast"),
    }
}

pub fn tone_from_mean(mean: Option<f64>) -> &'static str {
    match mean {
        None => "neutral",
        Some(value) if value >= 1.0 => "green",
        Some(value) if value >= 0.0 => "lime",
        Some(value) if value >= -1.0 => "amber",
        Some(_) => "red",
    }
}

pub fn order_color(ordering: bool) -> &'static str {
    if ordering {
        "green"
    } else {
        "orange"
    }
}

pub fn friendly_device(user_agent: &str) -> String {
    let browser = if user_agent.contains("Edg/") {
        "Edge"
    } else if user_agent.contains("Chrome/") {
        "Chrome"
    } else if user_agent.contains("Firefox/") {
        "Firefox"
    } else if user_agent.contains("Safari/") {
        "Safari"
    } else {
        "浏览器"
    };
    let os = if user_agent.contains("Windows") {
        "Windows"
    } else if user_agent.contains("Mac OS") || user_agent.contains("Macintosh") {
        "macOS"
    } else if user_agent.contains("Android") {
        "Android"
    } else if user_agent.contains("iPhone") || user_agent.contains("iPad") {
        "iOS"
    } else if user_agent.contains("Linux") {
        "Linux"
    } else {
        "设备"
    };
    format!("{os} · {browser}")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotFlags {
    #[serde(default = "default_true")]
    pub breakfast: bool,
    #[serde(default = "default_true")]
    pub lunch: bool,
    #[serde(default = "default_true")]
    pub dinner: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SlotFlags {
    fn default() -> Self {
        Self {
            breakfast: true,
            lunch: true,
            dinner: true,
        }
    }
}

impl SlotFlags {
    pub fn get(&self, slot: &str) -> bool {
        match slot {
            "breakfast" => self.breakfast,
            "lunch" => self.lunch,
            "dinner" => self.dinner,
            _ => false,
        }
    }

    pub fn set(&mut self, slot: &str, value: bool) {
        match slot {
            "breakfast" => self.breakfast = value,
            "lunch" => self.lunch = value,
            "dinner" => self.dinner = value,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeekDay {
    pub weekday: u8,
    pub breakfast: bool,
    pub lunch: bool,
    pub dinner: bool,
}

impl WeekDay {
    pub fn get(&self, slot: &str) -> bool {
        match slot {
            "breakfast" => self.breakfast,
            "lunch" => self.lunch,
            "dinner" => self.dinner,
            _ => false,
        }
    }

    pub fn blank(weekday: u8) -> Self {
        Self {
            weekday,
            breakfast: false,
            lunch: false,
            dinner: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DayOverride {
    pub date: NaiveDate,
    pub slot: String,
    pub ordering: bool,
    #[serde(default)]
    pub canteen_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MealDsl {
    #[serde(default = "default_version")]
    pub version: i32,
    pub mode: String,
    #[serde(default)]
    pub anchor_date: Option<NaiveDate>,
    #[serde(default)]
    pub canteen_id: Option<Uuid>,
    #[serde(default)]
    pub slots: SlotFlags,
    #[serde(default)]
    pub week: Vec<WeekDay>,
    #[serde(default)]
    pub overrides: Vec<DayOverride>,
}

fn default_version() -> i32 {
    1
}

impl MealDsl {
    pub fn work2(anchor: NaiveDate, canteen_id: Option<Uuid>) -> Self {
        Self {
            version: 1,
            mode: "work2_rest4".into(),
            anchor_date: Some(anchor),
            canteen_id,
            slots: SlotFlags::default(),
            week: Vec::new(),
            overrides: Vec::new(),
        }
    }

    pub fn custom_blank(canteen_id: Option<Uuid>) -> Self {
        Self {
            version: 1,
            mode: "custom".into(),
            anchor_date: None,
            canteen_id,
            slots: SlotFlags {
                breakfast: false,
                lunch: false,
                dinner: false,
            },
            week: (1..=7).map(WeekDay::blank).collect(),
            overrides: Vec::new(),
        }
    }

    pub fn valid_mode(&self) -> bool {
        self.mode == "work2_rest4" || self.mode == "custom"
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedMeal {
    pub ordering: bool,
    pub canteen_id: Option<Uuid>,
    pub explicit: bool,
}

pub fn resolve_meal(dsl: &MealDsl, date: NaiveDate, slot: &str) -> ResolvedMeal {
    if let Some(item) = dsl
        .overrides
        .iter()
        .find(|item| item.date == date && item.slot == slot)
    {
        return ResolvedMeal {
            ordering: item.ordering,
            canteen_id: item.canteen_id.or(dsl.canteen_id),
            explicit: true,
        };
    }
    if dsl.mode == "work2_rest4" {
        let Some(anchor) = dsl.anchor_date else {
            return ResolvedMeal {
                ordering: false,
                canteen_id: dsl.canteen_id,
                explicit: false,
            };
        };
        if is_work_day(date, anchor) {
            return ResolvedMeal {
                ordering: dsl.slots.get(slot),
                canteen_id: dsl.canteen_id,
                explicit: false,
            };
        }
        return ResolvedMeal {
            ordering: false,
            canteen_id: dsl.canteen_id,
            explicit: false,
        };
    }
    let weekday = date.weekday().number_from_monday() as u8;
    let ordering = dsl
        .week
        .iter()
        .find(|day| day.weekday == weekday)
        .map(|day| day.get(slot))
        .unwrap_or(false);
    ResolvedMeal {
        ordering,
        canteen_id: dsl.canteen_id,
        explicit: false,
    }
}

pub fn upsert_override(
    dsl: &mut MealDsl,
    date: NaiveDate,
    slot: &str,
    ordering: bool,
    canteen_id: Option<Uuid>,
) {
    if let Some(item) = dsl
        .overrides
        .iter_mut()
        .find(|item| item.date == date && item.slot == slot)
    {
        item.ordering = ordering;
        if canteen_id.is_some() {
            item.canteen_id = canteen_id;
        }
        return;
    }
    dsl.overrides.push(DayOverride {
        date,
        slot: slot.to_string(),
        ordering,
        canteen_id,
    });
}

pub fn local_script_dsl(
    description: &str,
    anchor: Option<NaiveDate>,
    canteen_id: Option<Uuid>,
    today: NaiveDate,
) -> Result<MealDsl, String> {
    let trimmed = description.trim();
    if trimmed.starts_with('{') {
        let dsl: MealDsl = serde_json::from_str(trimmed).map_err(|_| "dsl_parse".to_string())?;
        if !dsl.valid_mode() {
            return Err("shift_kind_invalid".into());
        }
        return Ok(dsl);
    }
    if trimmed.contains("上二休四") || trimmed.contains("work2_rest4") {
        return Ok(MealDsl::work2(anchor.unwrap_or(today), canteen_id));
    }
    if trimmed.contains("上五休二") {
        return Err("shift_kind_invalid".into());
    }
    let mut dsl = MealDsl::custom_blank(canteen_id);
    for day in &mut dsl.week {
        day.lunch = true;
    }
    Ok(dsl)
}

pub fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

pub fn llm_request(
    kind: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
    prompt: &str,
) -> Result<LlmRequest, String> {
    match kind {
        "openai_chat" => {
            let base = if base_url.is_empty() {
                "https://api.openai.com/v1"
            } else {
                base_url
            };
            Ok(LlmRequest {
                url: join_url(base, "chat/completions"),
                headers: vec![("authorization".into(), format!("Bearer {api_key}"))],
                body: json!({
                    "model": model,
                    "messages": [{"role": "user", "content": prompt}]
                }),
            })
        }
        "openai_responses" => {
            let base = if base_url.is_empty() {
                "https://api.openai.com/v1"
            } else {
                base_url
            };
            Ok(LlmRequest {
                url: join_url(base, "responses"),
                headers: vec![("authorization".into(), format!("Bearer {api_key}"))],
                body: json!({
                    "model": model,
                    "input": prompt
                }),
            })
        }
        "anthropic" => {
            let base = if base_url.is_empty() {
                "https://api.anthropic.com"
            } else {
                base_url
            };
            Ok(LlmRequest {
                url: join_url(base, "v1/messages"),
                headers: vec![
                    ("x-api-key".into(), api_key.to_string()),
                    ("anthropic-version".into(), "2023-06-01".into()),
                ],
                body: json!({
                    "model": model,
                    "max_tokens": 1024,
                    "messages": [{"role": "user", "content": prompt}]
                }),
            })
        }
        _ => Err("channel_kind_invalid".into()),
    }
}

pub fn dsl_prompt(description: &str) -> String {
    format!(
        "把下面的班制描述转换成 JSON。只输出 JSON，不要解释。\
mode 只能是 work2_rest4 或 custom。work2_rest4 需要 anchor_date（YYYY-MM-DD）。\
custom 的 week 是 weekday 1 到 7，每天 breakfast/lunch/dinner 布尔值。\
不要输出上五休二。描述：{description}"
    )
}

pub fn parse_dsl_text(text: &str) -> Result<MealDsl, String> {
    let trimmed = text.trim();
    if let Ok(dsl) = serde_json::from_str::<MealDsl>(trimmed) {
        if dsl.valid_mode() {
            return Ok(dsl);
        }
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(content) = value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
        {
            return parse_dsl_text(content);
        }
        if let Some(content) = value.get("output_text").and_then(Value::as_str) {
            return parse_dsl_text(content);
        }
        if let Some(items) = value.get("output").and_then(Value::as_array) {
            for item in items {
                if let Some(parts) = item.get("content").and_then(Value::as_array) {
                    for part in parts {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            if let Ok(dsl) = parse_dsl_text(text) {
                                return Ok(dsl);
                            }
                        }
                    }
                }
            }
        }
        if let Some(parts) = value.get("content").and_then(Value::as_array) {
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    if let Ok(dsl) = parse_dsl_text(text) {
                        return Ok(dsl);
                    }
                }
            }
        }
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            if let Ok(dsl) = serde_json::from_str::<MealDsl>(&trimmed[start..=end]) {
                if dsl.valid_mode() {
                    return Ok(dsl);
                }
            }
        }
    }
    Err("dsl_parse".into())
}

const REDACTED_KEYS: &[&str] = &[
    "password",
    "password_hash",
    "token",
    "session",
    "secret",
    "totp",
    "invite",
    "invite_code",
    "code",
    "magic",
    "magic_link",
    "api_key",
    "key",
    "llm_key",
    "image",
    "image_bytes",
    "authorization",
    "poll_token",
];

pub fn sanitize_summary(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut clean = Map::new();
            for (key, item) in map {
                let lower = key.to_ascii_lowercase();
                if REDACTED_KEYS.iter().any(|banned| lower.contains(banned)) {
                    continue;
                }
                clean.insert(key, sanitize_summary(item));
            }
            Value::Object(clean)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_summary).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> DateTime<Utc> {
        shanghai_at(
            NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            NaiveTime::from_hms_opt(hh, mm, 0).unwrap(),
        )
    }

    #[test]
    fn work2_rest4_cycle_and_rest_days() {
        let anchor = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        assert!(is_work_day(anchor, anchor));
        assert!(is_work_day(anchor + Duration::days(1), anchor));
        assert!(!is_work_day(anchor + Duration::days(2), anchor));
        assert!(!is_work_day(anchor + Duration::days(5), anchor));
        assert!(is_work_day(anchor + Duration::days(6), anchor));
    }

    #[test]
    fn next_slot_walks_the_week() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        assert_eq!(next_slot(monday, "breakfast"), (monday, "lunch"));
        assert_eq!(next_slot(monday, "lunch"), (monday, "dinner"));
        assert_eq!(
            next_slot(monday, "dinner"),
            (NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(), "breakfast")
        );
        let sunday = NaiveDate::from_ymd_opt(2026, 10, 11).unwrap();
        assert_eq!(
            next_slot(sunday, "dinner"),
            (NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(), "breakfast")
        );
        assert_eq!(
            week_start(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()),
            monday
        );
    }

    #[test]
    fn lock_and_score_window_use_configured_times() {
        let lock = NaiveTime::from_hms_opt(15, 4, 0).unwrap();
        let meal = NaiveDate::from_ymd_opt(2026, 10, 11).unwrap();
        assert!(!meal_is_locked(meal, lock, at(2026, 10, 10, 15, 3)));
        assert!(meal_is_locked(meal, lock, at(2026, 10, 10, 15, 4)));
        let after = NaiveTime::from_hms_opt(12, 30, 0).unwrap();
        assert!(!score_window_open(meal, after, at(2026, 10, 11, 12, 29)));
        assert!(score_window_open(meal, after, at(2026, 10, 11, 12, 30)));
        assert!(score_window_open(meal, after, at(2026, 10, 18, 12, 29)));
        assert!(!score_window_open(meal, after, at(2026, 10, 18, 12, 30)));
    }

    #[test]
    fn tone_is_higher_greener_without_a_number() {
        assert_eq!(tone_from_mean(None), "neutral");
        assert_eq!(tone_from_mean(Some(1.2)), "green");
        assert_eq!(tone_from_mean(Some(0.2)), "lime");
        assert_eq!(tone_from_mean(Some(-0.4)), "amber");
        assert_eq!(tone_from_mean(Some(-1.4)), "red");
        assert_eq!(order_color(true), "green");
        assert_eq!(order_color(false), "orange");
    }

    #[test]
    fn local_script_rejects_five_two_and_builds_work2() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        assert!(local_script_dsl("上五休二", None, None, today).is_err());
        let dsl = local_script_dsl("上二休四", Some(today), None, today).unwrap();
        assert_eq!(dsl.mode, "work2_rest4");
        assert!(!resolve_meal(&dsl, today + Duration::days(2), "lunch").ordering);
        assert!(resolve_meal(&dsl, today, "lunch").ordering);
    }

    #[test]
    fn llm_request_shapes() {
        let chat = llm_request(
            "openai_chat",
            "https://api.deepseek.com/v1/",
            "m",
            "secret",
            "hi",
        )
        .unwrap();
        assert_eq!(chat.url, "https://api.deepseek.com/v1/chat/completions");
        assert!(chat
            .headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == "Bearer secret"));
        let responses = llm_request("openai_responses", "", "m", "secret", "hi").unwrap();
        assert_eq!(responses.url, "https://api.openai.com/v1/responses");
        let anthropic = llm_request("anthropic", "", "m", "secret", "hi").unwrap();
        assert_eq!(anthropic.url, "https://api.anthropic.com/v1/messages");
        assert!(anthropic.headers.iter().any(|(k, _)| k == "x-api-key"));
    }

    #[test]
    fn sanitize_drops_secrets_and_image_bytes() {
        let clean = sanitize_summary(json!({
            "username": "a",
            "password": "nope",
            "api_key": "sk",
            "invite_code": "abc",
            "image_bytes": [1, 2, 3],
            "nested": {"token": "t", "action": "ok"}
        }));
        let text = clean.to_string();
        assert!(text.contains("username"));
        assert!(!text.contains("nope"));
        assert!(!text.contains("sk"));
        assert!(!text.contains("abc"));
        assert!(!text.contains("image_bytes"));
    }

    #[test]
    fn friendly_name_reads_edge_before_chrome() {
        let name = friendly_device("Mozilla/5.0 (Windows NT 10.0) Chrome/120.0 Edg/120.0");
        assert_eq!(name, "Windows · Edge");
    }
}
