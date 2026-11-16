-- Temporary lock after repeated sign-in failures. The password itself is never stored here.

ALTER TABLE users
    ADD COLUMN failed_login_count integer NOT NULL DEFAULT 0,
    ADD COLUMN locked_until timestamptz,
    ADD CONSTRAINT users_failed_login_count_chk CHECK (failed_login_count >= 0);
