SELECT name
FROM mcservers
WHERE guild_id = ?
    AND name LIKE ? || '%'
ORDER BY name
LIMIT 10