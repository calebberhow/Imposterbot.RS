-- Data for minecraft servers hosted
CREATE TABLE IF NOT EXISTS mcservers (
    guild_id INTEGER NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE,
    address TEXT NOT NULL,
    port INTEGER NOT NULL,
    PRIMARY KEY (guild_id, name)
);

-- Channel to send welcome / goodbye messages to
CREATE TABLE IF NOT EXISTS welcome_channel (
    guild_id INTEGER NOT NULL PRIMARY KEY,
    channel_id INTEGER NOT NULL
);

-- List of roles to add to member on guild join
CREATE TABLE IF NOT EXISTS member_roles_on_join (
    guild_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    PRIMARY KEY (guild_id, role_id)
);
