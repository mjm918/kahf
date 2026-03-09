CREATE TABLE audit_log (
    id         UUID NOT NULL DEFAULT gen_random_uuid(),
    ts         TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_id    UUID,
    action     TEXT NOT NULL,
    resource   TEXT,
    outcome    TEXT NOT NULL,
    detail     JSONB,
    ip_addr    INET,
    user_agent TEXT
);

SELECT create_hypertable('audit_log', 'ts');

ALTER TABLE audit_log SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'action',
    timescaledb.compress_orderby = 'ts ASC'
);
SELECT add_compression_policy('audit_log', INTERVAL '7 days');

CREATE INDEX idx_audit_user   ON audit_log (user_id, ts DESC);
CREATE INDEX idx_audit_action ON audit_log (action, ts DESC);
