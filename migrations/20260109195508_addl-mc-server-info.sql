-- Add migration script here
ALTER TABLE mcservers
ADD COLUMN version TEXT NOT NULL DEFAULT '';
ALTER TABLE mcservers
ADD COLUMN modpack TEXT NOT NULL DEFAULT '';
ALTER TABLE mcservers
ADD COLUMN custom_description TEXT NOT NULL DEFAULT '';
ALTER TABLE mcservers
ADD COLUMN instructions TEXT NOT NULL DEFAULT '';
ALTER TABLE mcservers
ADD COLUMN thumbnail TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_mcservers_guild_name ON mcservers (guild_id, name);