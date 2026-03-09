CREATE TABLE notification_preferences (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel     TEXT NOT NULL,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    snoozed_until TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, channel)
);
