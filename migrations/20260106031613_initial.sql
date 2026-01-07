-- Add migration script here
CREATE TABLE IF NOT EXISTS mcservers (
    guild_id INTEGER NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE,
    address TEXT NOT NULL,
    port INTEGER NOT NULL,
    PRIMARY KEY (guild_id, name)
)