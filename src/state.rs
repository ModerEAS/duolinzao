use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;
use webauthn_rs::prelude::{PasskeyAuthentication, PasskeyRegistration, Webauthn, WebauthnBuilder};

use crate::config::AppConfig;
use crate::domain::{llm_request, parse_dsl_text, MealDsl};
use crate::error::AppError;

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub struct ManualClock {
    now: Mutex<DateTime<Utc>>,
}

impl ManualClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    pub fn set(&self, now: DateTime<Utc>) {
        *self.now.lock().expect("clock") = now;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock")
    }
}

// 测试时钟：生产 clock，平常与系统钟一致，仅测试环境入口可拨偏置。
// 后端所有锁单/评分窗判断都走 state.clock.now()，拨钟即拨业务时间。
// 入口见 routes::test_clock_*，无 DLZ_ALLOW_TEST_CLOCK=1 时 404。
pub struct OverrideClock {
    now: Mutex<Option<DateTime<Utc>>>,
}

impl OverrideClock {
    pub fn new() -> Self {
        Self { now: Mutex::new(None) }
    }

    pub fn set(&self, now: Option<DateTime<Utc>>) {
        *self.now.lock().expect("clock") = now;
    }

    pub fn get(&self) -> Option<DateTime<Utc>> {
        *self.now.lock().expect("clock")
    }
}

impl Default for OverrideClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for OverrideClock {
    fn now(&self) -> DateTime<Utc> {
        self.get().unwrap_or_else(Utc::now)
    }
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
}

pub struct CapturingMailer {
    pub messages: Mutex<Vec<(String, String, String)>>,
}

impl CapturingMailer {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
        }
    }
}

impl Default for CapturingMailer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Mailer for CapturingMailer {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        self.messages.lock().expect("mail").push((
            to.to_string(),
            subject.to_string(),
            body.to_string(),
        ));
        Ok(())
    }
}

pub struct SmtpMailer {
    host: String,
    port: u16,
    username: String,
    password: String,
    from: String,
}

impl SmtpMailer {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            host: config.smtp_host.clone(),
            port: config.smtp_port,
            username: config.smtp_username.clone(),
            password: config.smtp_password.clone(),
            from: config.smtp_from.clone(),
        }
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        let email = Message::builder()
            .from(self.from.parse().map_err(|err| format!("from: {err}"))?)
            .to(to.parse().map_err(|err| format!("to: {err}"))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|err| err.to_string())?;
        let mut transport =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host).port(self.port);
        if !self.username.is_empty() {
            transport = transport.credentials(Credentials::new(
                self.username.clone(),
                self.password.clone(),
            ));
        }
        transport
            .build()
            .send(email)
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[async_trait]
pub trait TemplateBackend: Send + Sync {
    fn record(&self, kind: &str) {
        let _ = kind;
    }

    async fn complete(
        &self,
        kind: &str,
        base_url: &str,
        model: &str,
        api_key: &str,
        prompt: &str,
    ) -> Result<String, String>;
}

#[derive(Clone)]
pub struct ScriptedBackend {
    pub calls: Arc<Mutex<Vec<String>>>,
    pub fail_kinds: Arc<Mutex<Vec<String>>>,
    pub response: Arc<Mutex<Option<String>>>,
}

impl ScriptedBackend {
    pub fn failing_then_script() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            fail_kinds: Arc::new(Mutex::new(vec![
                "openai_chat".into(),
                "openai_responses".into(),
                "anthropic".into(),
            ])),
            response: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl TemplateBackend for ScriptedBackend {
    fn record(&self, kind: &str) {
        self.calls.lock().expect("calls").push(kind.to_string());
    }

    async fn complete(
        &self,
        kind: &str,
        _base_url: &str,
        _model: &str,
        _api_key: &str,
        _prompt: &str,
    ) -> Result<String, String> {
        if self
            .fail_kinds
            .lock()
            .expect("fail")
            .iter()
            .any(|item| item == kind)
        {
            return Err(format!("{kind}_failed"));
        }
        if let Some(body) = self.response.lock().expect("response").clone() {
            return Ok(body);
        }
        Ok(r#"{"mode":"custom","week":[{"weekday":1,"breakfast":false,"lunch":true,"dinner":false},{"weekday":2,"breakfast":false,"lunch":true,"dinner":false},{"weekday":3,"breakfast":false,"lunch":true,"dinner":false},{"weekday":4,"breakfast":false,"lunch":true,"dinner":false},{"weekday":5,"breakfast":false,"lunch":true,"dinner":false},{"weekday":6,"breakfast":false,"lunch":true,"dinner":false},{"weekday":7,"breakfast":false,"lunch":true,"dinner":false}],"overrides":[]}"#.into())
    }
}

pub struct HttpTemplateBackend {
    client: Client,
}

impl HttpTemplateBackend {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("http client");
        Self { client }
    }
}

#[async_trait]
impl TemplateBackend for HttpTemplateBackend {
    async fn complete(
        &self,
        kind: &str,
        base_url: &str,
        model: &str,
        api_key: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let request = llm_request(kind, base_url, model, api_key, prompt)?;
        let mut builder = self.client.post(&request.url).json(&request.body);
        for (key, value) in request.headers {
            builder = builder.header(key, value);
        }
        let response = builder.send().await.map_err(|err| err.to_string())?;
        let status = response.status();
        let text = response.text().await.map_err(|err| err.to_string())?;
        if !status.is_success() {
            return Err(format!("http {}", status.as_u16()));
        }
        let _dsl: MealDsl = parse_dsl_text(&text)?;
        Ok(text)
    }
}

pub enum Challenge {
    Register {
        account_id: Uuid,
        state: PasskeyRegistration,
        expires_at: DateTime<Utc>,
    },
    Authenticate {
        state: PasskeyAuthentication,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Default)]
pub struct ChallengeStore {
    inner: AsyncMutex<HashMap<Uuid, Challenge>>,
}

impl ChallengeStore {
    pub async fn insert(&self, id: Uuid, challenge: Challenge) {
        self.inner.lock().await.insert(id, challenge);
    }

    pub async fn take(&self, id: Uuid, now: DateTime<Utc>) -> Option<Challenge> {
        let mut guard = self.inner.lock().await;
        guard.retain(|_, challenge| match challenge {
            Challenge::Register { expires_at, .. } | Challenge::Authenticate { expires_at, .. } => {
                *expires_at > now
            }
        });
        let challenge = guard.remove(&id)?;
        let alive = match &challenge {
            Challenge::Register { expires_at, .. } | Challenge::Authenticate { expires_at, .. } => {
                *expires_at > now
            }
        };
        if alive {
            Some(challenge)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: std::sync::Arc<AppConfig>,
    pub clock: std::sync::Arc<dyn Clock>,
    pub test_clock: std::sync::Arc<OverrideClock>,
    pub mailer: std::sync::Arc<dyn Mailer>,
    pub templates: std::sync::Arc<dyn TemplateBackend>,
    pub webauthn: std::sync::Arc<Webauthn>,
    pub challenges: std::sync::Arc<ChallengeStore>,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        config: AppConfig,
        clock: std::sync::Arc<dyn Clock>,
        mailer: std::sync::Arc<dyn Mailer>,
        templates: std::sync::Arc<dyn TemplateBackend>,
    ) -> Result<Self, AppError> {
        let webauthn = build_webauthn(&config)?;
        Ok(Self {
            pool,
            config: std::sync::Arc::new(config),
            clock,
            test_clock: std::sync::Arc::new(OverrideClock::new()),
            mailer,
            templates,
            webauthn: std::sync::Arc::new(webauthn),
            challenges: std::sync::Arc::new(ChallengeStore::default()),
        })
    }

    pub fn production(pool: PgPool, config: AppConfig) -> Result<Self, AppError> {
        let mailer: std::sync::Arc<dyn Mailer> = std::sync::Arc::new(SmtpMailer::new(&config));
        let templates: std::sync::Arc<dyn TemplateBackend> =
            std::sync::Arc::new(HttpTemplateBackend::new());
        // 生产 clock 即测试时钟载体：未拨时行为与系统钟完全一致。
        let test_clock = std::sync::Arc::new(OverrideClock::new());
        let clock: std::sync::Arc<dyn Clock> = test_clock.clone();
        let mut state = Self::new(pool, config, clock, mailer, templates)?;
        state.test_clock = test_clock;
        Ok(state)
    }
}

fn build_webauthn(config: &AppConfig) -> Result<Webauthn, AppError> {
    let origin = url::Url::parse(&config.webauthn_origin)
        .map_err(|_| AppError::bad("config", "WebAuthn 来源无效"))?;
    WebauthnBuilder::new(&config.webauthn_rp_id, &origin)
        .and_then(|builder| builder.rp_name(&config.webauthn_rp_name).build())
        .map_err(|err| {
            tracing::error!(error = ?err, "webauthn");
            AppError::bad("config", "WebAuthn 配置无效")
        })
}
