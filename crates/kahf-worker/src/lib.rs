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
//! used for audit logging and Redis queue namespacing. Blanket-implemented
//! via a derive-free approach — just implement `JOB_TYPE` as an associated
//! constant.
//!
//! ## JobProducer
//!
//! Cloneable handle held in `AppState`. Exposes a single generic
//! `enqueue<J: Job>(job)` method that works with any job type. Internally
//! creates `RedisStorage<J>` from a shared connection pool. Each enqueue
//! generates a UUID job ID, records an `Enqueued` audit entry, and pushes
//! the job to Redis.
//!
//! ## start_workers
//!
//! Creates apalis workers for all registered job types and returns a
//! `Monitor`. Adding a new job type requires only: defining the struct,
//! implementing `Job`, writing a handler function, and registering the
//! worker in `start_workers`.
//!
//! ## jobs
//!
//! Job struct definitions organized by domain (email, etc).
//!
//! ## handlers
//!
//! Job handler functions that perform the actual work.
//!
//! ## audit
//!
//! `JobAuditor` for recording state transitions to TimescaleDB.

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
use kahf_email::EmailSender;

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

pub async fn start_workers(
    redis_url: &str,
    pool: PgPool,
    mailer: Arc<dyn EmailSender>,
) -> eyre::Result<Monitor> {
    let conn = apalis_redis::connect(redis_url)
        .await
        .map_err(|e| eyre::eyre!("failed to connect to Redis for workers: {}", e))?;

    let auditor = JobAuditor::new(pool);

    let otp_config = apalis_redis::Config::default().set_namespace("kahflane:SendOtpEmail");
    let reset_config = apalis_redis::Config::default().set_namespace("kahflane:SendPasswordResetEmail");
    let invite_config = apalis_redis::Config::default().set_namespace("kahflane:SendInviteEmail");

    let otp_worker = WorkerBuilder::new("email-otp")
        .data(mailer.clone())
        .data(auditor.clone())
        .enable_tracing()
        .backend(RedisStorage::<jobs::SendOtpEmail>::new_with_config(conn.clone(), otp_config))
        .build_fn(handlers::email::handle_send_otp);

    let reset_worker = WorkerBuilder::new("email-reset")
        .data(mailer.clone())
        .data(auditor.clone())
        .enable_tracing()
        .backend(RedisStorage::<jobs::SendPasswordResetEmail>::new_with_config(conn.clone(), reset_config))
        .build_fn(handlers::email::handle_send_password_reset);

    let invite_worker = WorkerBuilder::new("email-invite")
        .data(mailer)
        .data(auditor)
        .enable_tracing()
        .backend(RedisStorage::<jobs::SendInviteEmail>::new_with_config(conn, invite_config))
        .build_fn(handlers::email::handle_send_invite);

    let monitor = Monitor::new()
        .register(otp_worker)
        .register(reset_worker)
        .register(invite_worker);

    Ok(monitor)
}
