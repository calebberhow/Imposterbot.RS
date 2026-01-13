UPDATE mcservers
SET address = COALESCE(?, address),
    port = COALESCE(?, port),
    version = COALESCE(?, version),
    modpack = COALESCE(?, modpack),
    custom_description = COALESCE(?, custom_description),
    instructions = COALESCE(?, instructions),
    thumbnail = COALESCE(?, thumbnail)
WHERE guild_id = ?
    AND name = ?