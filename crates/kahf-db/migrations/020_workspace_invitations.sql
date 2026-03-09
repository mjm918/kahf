ALTER TABLE invitations ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;

DROP INDEX IF EXISTS idx_invitations_pending;
CREATE UNIQUE INDEX idx_invitations_pending ON invitations (email, workspace_id) WHERE accepted = false;
