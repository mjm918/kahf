CREATE TABLE crdt_docs (
    doc_id        UUID PRIMARY KEY,
    workspace_id  UUID NOT NULL,
    state         BYTEA NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL
);
