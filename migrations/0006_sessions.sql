-- Sessions table managed by tower-sessions-sqlx-store
CREATE TABLE IF NOT EXISTS tower_sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    data        BLOB NOT NULL,
    expiry_date INTEGER NOT NULL
);
