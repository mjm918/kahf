CREATE TABLE invitations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT NOT NULL,
    invited_by  UUID NOT NULL REFERENCES users(id),
    token       TEXT UNIQUE NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    accepted    BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invitations_token ON invitations (token) WHERE accepted = false;
CREATE UNIQUE INDEX idx_invitations_pending ON invitations (email) WHERE accepted = false;
