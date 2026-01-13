SELECT address,
    port,
    version,
    modpack,
    custom_description,
    instructions,
    thumbnail
FROM mcservers
WHERE guild_id = ?
    AND name = ?