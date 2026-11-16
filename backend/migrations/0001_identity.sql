-- Identity, access, preferences, and audit.

CREATE EXTENSION IF NOT EXISTS citext;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email citext NOT NULL UNIQUE,
    password_hash text NOT NULL,
    display_name text NOT NULL,
    avatar_storage_key text,
    is_active boolean NOT NULL DEFAULT true,
    totp_enabled boolean NOT NULL DEFAULT false,
    totp_secret_encrypted text,
    last_login_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT users_display_name_chk CHECK (length(trim(display_name)) > 0)
);

CREATE TRIGGER users_set_updated_at
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE roles (
    id uuid PRIMARY KEY,
    name text NOT NULL UNIQUE,
    description text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE permissions (
    id uuid PRIMARY KEY,
    key text NOT NULL UNIQUE,
    description text NOT NULL DEFAULT ''
);

CREATE TABLE user_roles (
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE role_permissions (
    role_id uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_id uuid NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    user_agent text,
    ip_address inet,
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_active_idx
    ON sessions (user_id)
    WHERE revoked_at IS NULL;

CREATE TABLE user_preferences (
    user_id uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    theme text NOT NULL DEFAULT 'system',
    timezone text NOT NULL DEFAULT 'UTC',
    density text NOT NULL DEFAULT 'comfortable',
    default_mailbox_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT user_preferences_theme_chk CHECK (theme IN ('light', 'dark', 'system')),
    CONSTRAINT user_preferences_density_chk CHECK (density IN ('comfortable', 'compact'))
);

CREATE TRIGGER user_preferences_set_updated_at
BEFORE UPDATE ON user_preferences
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE audit_logs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_user_id uuid REFERENCES users (id) ON DELETE SET NULL,
    action text NOT NULL,
    entity_type text,
    entity_id uuid,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    ip_address inet,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX audit_logs_actor_created_idx
    ON audit_logs (actor_user_id, created_at DESC);

CREATE INDEX audit_logs_entity_idx
    ON audit_logs (entity_type, entity_id);

INSERT INTO roles (id, name, description) VALUES
    ('10000000-0000-4000-8000-000000000001', 'owner', 'Full control of the company mail platform'),
    ('10000000-0000-4000-8000-000000000002', 'administrator', 'Manages people, mailboxes, and audit history'),
    ('10000000-0000-4000-8000-000000000003', 'member', 'Uses mailboxes they have been granted');

INSERT INTO permissions (id, key, description) VALUES
    ('20000000-0000-4000-8000-000000000001', 'mail.read', 'Read messages in granted mailboxes'),
    ('20000000-0000-4000-8000-000000000002', 'mail.send', 'Send messages from granted mailboxes'),
    ('20000000-0000-4000-8000-000000000003', 'mail.organize', 'Move, label, archive, and delete messages'),
    ('20000000-0000-4000-8000-000000000004', 'mailbox.manage', 'Create and configure mailboxes'),
    ('20000000-0000-4000-8000-000000000005', 'contacts.manage', 'Manage personal contacts'),
    ('20000000-0000-4000-8000-000000000006', 'users.manage', 'Manage user accounts and roles'),
    ('20000000-0000-4000-8000-000000000007', 'audit.read', 'Read the audit log');

INSERT INTO role_permissions (role_id, permission_id)
SELECT '10000000-0000-4000-8000-000000000001'::uuid, id FROM permissions;

INSERT INTO role_permissions (role_id, permission_id)
SELECT '10000000-0000-4000-8000-000000000002'::uuid, id FROM permissions;

INSERT INTO role_permissions (role_id, permission_id)
SELECT '10000000-0000-4000-8000-000000000003'::uuid, id
FROM permissions
WHERE key IN ('mail.read', 'mail.send', 'mail.organize', 'contacts.manage');
