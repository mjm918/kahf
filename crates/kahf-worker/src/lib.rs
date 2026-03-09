//! Background job processing for KahfLane using apalis + Redis.
//!
//! Provides a generic `JobProducer` that can enqueue any type implementing
//! the `Job` trait, and `start_workers` for spawning apalis workers.
//! All job state transitions are recorded in the `job_audit` TimescaleDB
//! hypertable for full audit trail.
//!
//! ## Job
//!
//! Trait that all job types must implement. Provides the job type name
//! used for audit logging and Redis queue namespacing.
//!
//! ## JobProducer
//!
//! Cloneable handle held in `AppState`. Exposes a single generic
//! `enqueue<J: Job>(job)` method that works with any job type.
//!
//! ## WorkerDeps
//!
//! Optional dependencies for notification channel workers. Telegram and
//! web push senders are `Option`-wrapped — workers are only registered
//! when the corresponding env vars are set.
//!
//! ## start_workers
//!
//! Creates apalis workers for all registered job types and returns a
//! `Monitor`. Channels without configured senders are skipped.
//!
//! ## jobs
//!
//! Job struct definitions organized by domain (email, telegram, web_push,
//! notification, audit).
//!
//! ## handlers
//!
//! Job handler functions that perform the actual work.
//!
//! ## audit
//!
//! `JobAuditor` for recording job state transitions to TimescaleDB.

pub mod audit;
pub mod handlers;
pub mod jobs;

use std::sync::Arc;

use apalis::prelude::*;
use apalis_redis::{ConnectionManager, RedisStorage};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::PgPool;
use uuid::Uuid;

use audit::{JobAuditor, JobStatus};
use kahf_notify::{EmailSender, TelegramSender, WebPushSender};

pub trait Job: Serialize + DeserializeOwned + Send + Sync + Unpin + Clone + 'static {
    const JOB_TYPE: &'static str;
}

#[derive(Clone)]
pub struct JobProducer {
    conn: ConnectionManager,
    namespace_prefix: String,
    auditor: JobAuditor,
}

impl JobProducer {
    pub async fn new(redis_url: &str, pool: PgPool) -> eyre::Result<Self> {
        let conn = apalis_redis::connect(redis_url)
            .await
            .map_err(|e| eyre::eyre!("failed to connect to Redis: {}", e))?;

        Ok(Self {
            conn,
            namespace_prefix: "kahflane".to_owned(),
            auditor: JobAuditor::new(pool),
        })
    }

    pub async fn noop(redis_url: &str) -> eyre::Result<Self> {
        let conn = apalis_redis::connect(redis_url)
            .await
            .map_err(|e| eyre::eyre!("failed to connect to Redis: {}", e))?;

        Ok(Self {
            conn,
            namespace_prefix: "kahflane-test".to_owned(),
            auditor: JobAuditor::noop(),
        })
    }

    pub async fn enqueue<J: Job>(&self, job: J) -> eyre::Result<Uuid> {
        let job_id = Uuid::new_v4();
        let payload = serde_json::to_value(&job)?;

        self.auditor
            .record(job_id, J::JOB_TYPE, JobStatus::Enqueued, payload, None, 1)
            .await;

        let ns = format!("{}:{}", self.namespace_prefix, J::JOB_TYPE);
        let config = apalis_redis::Config::default().set_namespace(&ns);
        let mut storage: RedisStorage<J> =
            RedisStorage::new_with_config(self.conn.clone(), config);

        storage
            .push(job)
            .await
            .map_err(|e| eyre::eyre!("failed to enqueue {} job: {}", J::JOB_TYPE, e))?;

        tracing::info!(job_id = %job_id, job_type = J::JOB_TYPE, "job enqueued");
        Ok(job_id)
    }
}

pub struct WorkerDeps {
    pub mailer: Arc<dyn EmailSender>,
    pub telegram: Option<Arc<TelegramSender>>,
    pub web_push: Option<Arc<WebPushSender>>,
}

pub async fn start_workers(
    redis_url: &str,
    pool: PgPool,
    deps: WorkerDeps,
) -> eyre::Result<Monitor> {
    let conn = apalis_redis::connect(redis_url)
        .await
        .map_err(|e| eyre::eyre!("failed to connect to Redis for workers: {}", e))?;

    let auditor = JobAuditor::new(pool.clone());

    let otp_config = apalis_redis::Config::default().set_namespace("kahflane:SendOtpEmail");
    let reset_config = apalis_redis::Config::default().set_namespace("kahflane:SendPasswordResetEmail");
    let invite_config = apalis_redis::Config::default().set_namespace("kahflane:SendInviteEmail");
    let audit_config = apalis_redis::Config::default().set_namespace("kahflane:AuditLog");
    let notif_config = apalis_redis::Config::default().set_namespace("kahflane:CreateInAppNotification");

    let otp_worker = WorkerBuilder::new("email-otp")
        .data(deps.mailer.clone())
        .data(auditor.clone())
        .enable_tracing()
        .backend(RedisStorage::<jobs::SendOtpEmail>::new_with_config(conn.clone(), otp_config))
        .build_fn(handlers::email::handle_send_otp);

    let reset_worker = WorkerBuilder::new("email-reset")
        .data(deps.mailer.clone())
        .data(auditor.clone())
        .enable_tracing()
        .backend(RedisStorage::<jobs::SendPasswordResetEmail>::new_with_config(conn.clone(), reset_config))
        .build_fn(handlers::email::handle_send_password_reset);

    let invite_worker = WorkerBuilder::new("email-invite")
        .data(deps.mailer)
        .data(auditor.clone())
        .enable_tracing()
        .backend(RedisStorage::<jobs::SendInviteEmail>::new_with_config(conn.clone(), invite_config))
        .build_fn(handlers::email::handle_send_invite);

    let notif_worker = WorkerBuilder::new("in-app-notification")
        .data(pool.clone())
        .data(auditor.clone())
        .enable_tracing()
        .backend(RedisStorage::<jobs::CreateInAppNotification>::new_with_config(conn.clone(), notif_config))
        .build_fn(handlers::notification::handle_create_in_app_notification);

    let audit_worker = WorkerBuilder::new("audit-log")
        .data(pool)
        .enable_tracing()
        .backend(RedisStorage::<jobs::AuditLog>::new_with_config(conn.clone(), audit_config))
        .build_fn(handlers::audit::handle_audit_log);

    let mut monitor = Monitor::new()
        .register(otp_worker)
        .register(reset_worker)
        .register(invite_worker)
        .register(notif_worker)
        .register(audit_worker);

    if let Some(telegram) = deps.telegram {
        tracing::info!("registering Telegram notification worker");
        let tg_config = apalis_redis::Config::default().set_namespace("kahflane:SendTelegramMessage");
        let tg_worker = WorkerBuilder::new("telegram")
            .data(telegram)
            .data(auditor.clone())
            .enable_tracing()
            .backend(RedisStorage::<jobs::SendTelegramMessage>::new_with_config(conn.clone(), tg_config))
            .build_fn(handlers::telegram::handle_send_telegram);
        monitor = monitor.register(tg_worker);
    }

    if let Some(web_push) = deps.web_push {
        tracing::info!("registering web push notification worker");
        let wp_config = apalis_redis::Config::default().set_namespace("kahflane:SendWebPush");
        let wp_worker = WorkerBuilder::new("web-push")
            .data(web_push)
            .data(auditor)
            .enable_tracing()
            .backend(RedisStorage::<jobs::SendWebPush>::new_with_config(conn, wp_config))
            .build_fn(handlers::web_push::handle_send_web_push);
        monitor = monitor.register(wp_worker);
    }

    Ok(monitor)
}
