CREATE TABLE notifications (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    category    TEXT NOT NULL,
    read_at     TIMESTAMPTZ,
    data        JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user_unread
    ON notifications(user_id, created_at DESC)
    WHERE read_at IS NULL;

CREATE INDEX idx_notifications_user_created
    ON notifications(user_id, created_at DESC);
