INSERT INTO welcome_channel (guild_id, channel_id)
VALUES (?, ?)
ON CONFLICT (guild_id)
DO UPDATE SET
    guild_id = excluded.guild_id,
    channel_id = excluded.channel_id
