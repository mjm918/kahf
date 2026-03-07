CREATE TABLE entities (
    id            UUID PRIMARY KEY,
    workspace_id  UUID NOT NULL,
    type          TEXT NOT NULL,
    data          JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,
    created_by    UUID NOT NULL,
    deleted       BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_entities_ws_type ON entities (workspace_id, type) WHERE NOT deleted;
CREATE INDEX idx_entities_data    ON entities USING GIN (data);
