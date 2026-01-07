UPDATE mcservers 
    SET address = COALESCE(?, address), port = COALESCE(?, port) 
WHERE name = ? AND guild_id = ?