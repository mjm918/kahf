CREATE TABLE telegram_links (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    telegram_chat_id    BIGINT NOT NULL,
    telegram_username   TEXT,
    linked_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
