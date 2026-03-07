CREATE TABLE tx_log (
    id            UUID NOT NULL DEFAULT gen_random_uuid(),
    ts            TIMESTAMPTZ NOT NULL DEFAULT now(),
    workspace_id  UUID NOT NULL,
    user_id       UUID NOT NULL,
    op            TEXT NOT NULL,
    entity_type   TEXT NOT NULL,
    entity_id     UUID NOT NULL,
    data          JSONB NOT NULL,
    metadata      JSONB
);

SELECT create_hypertable('tx_log', 'ts');

ALTER TABLE tx_log SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'workspace_id, entity_id',
    timescaledb.compress_orderby = 'ts ASC'
);
SELECT add_compression_policy('tx_log', INTERVAL '7 days');

CREATE INDEX idx_tx_entity ON tx_log (entity_id, ts DESC);
CREATE INDEX idx_tx_type   ON tx_log (workspace_id, entity_type, ts DESC);
