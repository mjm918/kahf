CREATE TABLE job_audit (
    id          UUID DEFAULT gen_random_uuid(),
    job_id      UUID NOT NULL,
    job_type    TEXT NOT NULL,
    status      TEXT NOT NULL,
    payload     JSONB NOT NULL,
    error       TEXT,
    attempt     INT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

SELECT create_hypertable('job_audit', 'created_at');

CREATE INDEX idx_job_audit_job_id ON job_audit (job_id);
CREATE INDEX idx_job_audit_job_type ON job_audit (job_type);
CREATE INDEX idx_job_audit_status ON job_audit (status);
