-- Shared request budgets. A bucket resets when its window expires.

CREATE TABLE rate_limits (
    bucket text PRIMARY KEY,
    hits integer NOT NULL,
    window_started_at timestamptz NOT NULL,
    CONSTRAINT rate_limits_hits_chk CHECK (hits >= 0)
);
