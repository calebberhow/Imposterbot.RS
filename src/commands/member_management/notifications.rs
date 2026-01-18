/*!

Poise command declarations for the various app commands that may be used to modify member join and member leave notifications

Declarations ONLY, no real implementations.

*/

use poise::{
    CreateReply,
    serenity_prelude::{self as serenity, CreateEmbed},
};

use crate::{
    Context, Error,
    commands::member_management::notifications_implementation::{
        MemberEventConfigurer, NotificationType,
    },
    events::guild_member::{guild_member_add, guild_member_remove},
    infrastructure::{colors, ids::require_guild_id},
    poise_instrument, record_ctx_fields,
};

static HELP_DESCRIPTION: &'static str = r#"
This command configures the join and leave messages for this guild.

This command can be used to make incremental updates to a notification format, or to fully replace the format with the specified format (`/notify-member join full` or `/notify-member leave full` commands)."#;

static HELP_IMAGES: &'static str = r#"
There are 3 places where an image can appear in the message:
1. thumbnail: large and at the top right of the embed,
2. main image: large image at the bottom of the embed
3. author-icon: small and next to the embed author text,
4. footer-icon: small and next to the embed footer text.

To specify an image in one of these locations use one of the appropriate `_file` or `_url` fields (but not both).
The `_url` field allows you to specify a web url to the content, and the `_file` field allows you to upload media to Imposterbot directly.
"#;

static HELP_PLACEHOLDERS: &'static str = r#"
When sending the message, Imposterbot will replace the following items with their values:

- `{name}` -> username of the user
- `{mention}` -> @mention's the user: Available only for `/notify-member join` commands.
- `{user_avatar}` -> url of user's avatar: If placed in a _url field (`thumbnail_url`, `author_icon_url`, or `footer_icon_url`), it will be rendered as an image.
- `{member_count}` -> current member count of the guild
- `{online_member_count}` -> current number of online members in the guild

Note: discord does not allow entering line breaks in command parameters, but you can get around this with `\n`.
"#;

static HELP_EXAMPLES: &'static str = r#"
Configure join notification from scratch (try it out by copying the command directly into the message box!):
```
/notify-member join full content: Welcome, {mention}!  description: **{name}** has joined  thumbnail_file: <attachment>  author: 𝚆𝚎𝚕𝚌𝚘𝚖𝚎 𝚝𝚘 𝙲𝚘𝚣𝚢 𝙲𝚘𝚜𝚖𝚘𝚜!  footer: Member count: {member_count}
```

Clear embed description for leave notification:
```
/notify-member leave description
```

Update the plain-text content of the join notification:
```
/notify-member join content *howdy, pard'ner!*
```
"#;

static HELP_LIST: &'static str = r#"
- `/notify-member join full`
- `/notify-member join title`
- `/notify-member join content`
- `/notify-member join description`
- `/notify-member join author`
- `/notify-member join footer`
- `/notify-member join thumbnail`
- `/notify-member join image`
- `/notify-member join author-icon`
- `/notify-member join footer-icon`

- `/notify-member leave full`
- `/notify-member leave title`
- `/notify-member leave content`
- `/notify-member leave description`
- `/notify-member leave author`
- `/notify-member leave footer`
- `/notify-member leave thumbnail`
- `/notify-member leave image`
- `/notify-member leave author-icon`
- `/notify-member leave footer-icon`
"#;

#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only,
    category = "Management",
    rename = "notify-member",
    subcommands("CfgMemberJoin::group", "CfgMemberLeave::group", "help")
)]
pub async fn cfg_member_notification(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

poise_instrument! {
    /// Shows documentation about /notify-member commands
    #[poise::command(
        slash_command,
        required_permissions = "ADMINISTRATOR",
        default_member_permissions = "ADMINISTRATOR",
        guild_only,
        category = "Management"
    )]
    async fn help(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::default()
                    .color(colors::slate())
                    .title("Help for /notify-member")
                    .description(HELP_DESCRIPTION)
                    .field("**Images**", HELP_IMAGES, false)
                    .field("**Placeholders**", HELP_PLACEHOLDERS, false)
                    .field("**Examples**", HELP_EXAMPLES, false)
                    .field("**Command List**", HELP_LIST, false),
            )
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
}
/// Subcommands of cfg_member_notification for join events
///
/// Contains poise declarations, but implementations are defined in the MemberEventConfigurer trait
struct CfgMemberJoin;

impl MemberEventConfigurer for CfgMemberJoin {
    const NOTIFICATION_TYPE: NotificationType = NotificationType::Join;
}

impl CfgMemberJoin {
    #[poise::command(
        slash_command,
        required_permissions = "ADMINISTRATOR",
        default_member_permissions = "ADMINISTRATOR",
        guild_only,
        category = "Management",
        rename = "join",
        subcommands(
            "CfgMemberJoin::full",
            "CfgMemberJoin::content",
            "CfgMemberJoin::title",
            "CfgMemberJoin::description",
            "CfgMemberJoin::thumbnail",
            "CfgMemberJoin::image",
            "CfgMemberJoin::author",
            "CfgMemberJoin::author_icon",
            "CfgMemberJoin::footer",
            "CfgMemberJoin::footer_icon",
        )
    )]
    async fn group(_ctx: Context<'_>) -> Result<(), Error> {
        Ok(())
    }

    poise_instrument! {
        /// Provides all configuration options for when members join this guild.
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "full",
            category = "Management"
        )]
        async fn full(
            ctx: Context<'_>,
            #[description = "Plain-text content of the notification message"] content: Option<String>,
            #[description = "Embed title text"] title: Option<String>,
            #[description = "Embed description text"] description: Option<String>,
            #[description = "Embed thumbnail file upload"] thumbnail_file: Option<serenity::Attachment>,
            #[description = "Embed thumbnail web url"] thumbnail_url: Option<String>,
            #[description = "Embed image file upload"] image_file: Option<serenity::Attachment>,
            #[description = "Embed image web url"] image_url: Option<String>,
            #[description = "Embed author text"] author: Option<String>,
            #[description = "Embed author icon file upload"] author_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed author icon web url"] author_icon_url: Option<String>,
            #[description = "Embed footer text"] footer: Option<String>,
            #[description = "Embed footer icon file upload"] footer_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed footer icon web url"] footer_icon_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::full_impl(
                ctx,
                content,
                title,
                description,
                thumbnail_file,
                thumbnail_url,
                image_file,
                image_url,
                author,
                author_icon_file,
                author_icon_url,
                footer,
                footer_icon_file,
                footer_icon_url,
            )
            .await
        }

        // Configures the join notification content
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn content(
            ctx: Context<'_>,
            #[description = "Plain-text content of the notification message"] content: Option<String>, // param matches func name
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::content_impl(ctx, content).await
        }

        /// Configures the join notification embed title
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn title(
            ctx: Context<'_>,
            #[description = "Embed title text"] title: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::title_impl(ctx, title).await
        }

        /// Configures the join notification embed description
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn description(
            ctx: Context<'_>,
            #[description = "Embed description text"] description: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::description_impl(ctx, description).await
        }

        /// Configures the join notification embed thumbnail
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn thumbnail(
            ctx: Context<'_>,
            #[description = "Embed thumbnail file upload"] thumbnail_file: Option<serenity::Attachment>,
            #[description = "Embed thumbnail web url"] thumbnail_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::thumbnail_impl(ctx, thumbnail_file, thumbnail_url).await
        }

        /// Configures the join notification embed image
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn image(
            ctx: Context<'_>,
            #[description = "Embed image file upload"] image_file: Option<serenity::Attachment>,
            #[description = "Embed image web url"] image_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::image_impl(ctx, image_file, image_url).await
        }

        /// Configures the join notification embed author
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn author(
            ctx: Context<'_>,
            #[description = "Embed author text"] author: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::author_impl(ctx, author).await
        }

        /// Configures the join notification embed author icon
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "author-icon",
            category = "Management"
        )]
        async fn author_icon(
            ctx: Context<'_>,
            #[description = "Embed author icon file upload"] author_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed author icon web url"] author_icon_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::author_icon_impl(ctx, author_icon_file, author_icon_url).await
        }

        /// Configures the join notification embed footer
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn footer(
            ctx: Context<'_>,
            #[description = "Embed footer text"] footer: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::footer_impl(ctx, footer).await
        }

        /// Configures the join notification embed footer icon
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "footer-icon",
            category = "Management"
        )]
        async fn footer_icon(
            ctx: Context<'_>,
            #[description = "Embed footer icon file upload"] footer_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed footer icon web url"] footer_icon_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberJoin::footer_icon_impl(ctx, footer_icon_file, footer_icon_url).await
        }
    }
}

/// Subcommands of cfg_member_notification for leave events
///
/// Contains poise declarations, but implementations are defined in the MemberEventConfigurer trait
struct CfgMemberLeave;

impl MemberEventConfigurer for CfgMemberLeave {
    const NOTIFICATION_TYPE: NotificationType = NotificationType::Leave;
}

impl CfgMemberLeave {
    #[poise::command(
        slash_command,
        required_permissions = "ADMINISTRATOR",
        default_member_permissions = "ADMINISTRATOR",
        guild_only,
        rename = "leave",
        category = "Management",
        subcommands(
            "CfgMemberLeave::full",
            "CfgMemberLeave::content",
            "CfgMemberLeave::title",
            "CfgMemberLeave::description",
            "CfgMemberLeave::thumbnail",
            "CfgMemberLeave::image",
            "CfgMemberLeave::author",
            "CfgMemberLeave::author_icon",
            "CfgMemberLeave::footer",
            "CfgMemberLeave::footer_icon",
        )
    )]
    async fn group(_ctx: Context<'_>) -> Result<(), Error> {
        Ok(())
    }

    poise_instrument! {
        /// Provides all configuration options for when members leave this guild.
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "full",
            category = "Management"
        )]
        async fn full(
            ctx: Context<'_>,
            #[description = "Plain-text content of the notification message"] content: Option<String>,
            #[description = "Embed title text"] title: Option<String>,
            #[description = "Embed description text"] description: Option<String>,
            #[description = "Embed thumbnail file upload"] thumbnail_file: Option<serenity::Attachment>,
            #[description = "Embed thumbnail web url"] thumbnail_url: Option<String>,
            #[description = "Embed image file upload"] image_file: Option<serenity::Attachment>,
            #[description = "Embed image web url"] image_url: Option<String>,
            #[description = "Embed author text"] author: Option<String>,
            #[description = "Embed author icon file upload"] author_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed author icon web url"] author_icon_url: Option<String>,
            #[description = "Embed footer text"] footer: Option<String>,
            #[description = "Embed footer icon file upload"] footer_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed footer icon web url"] footer_icon_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::full_impl(
                ctx,
                content,
                title,
                description,
                thumbnail_file,
                thumbnail_url,
                image_file,
                image_url,
                author,
                author_icon_file,
                author_icon_url,
                footer,
                footer_icon_file,
                footer_icon_url,
            )
            .await
        }

        /// Configures the leave notification content
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "content",
            category = "Management"
        )]
        async fn content(
            ctx: Context<'_>,
            #[description = "Plain-text content of the notification message"] content: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::content_impl(ctx, content).await
        }

        /// Configures the leave notification embed title
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn title(
            ctx: Context<'_>,
            #[description = "Embed title text"] title: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::title_impl(ctx, title).await
        }

        /// Configures the leave notification embed description
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "description",
            category = "Management"
        )]
        async fn description(
            ctx: Context<'_>,
            #[description = "Embed description text"] description: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::description_impl(ctx, description).await
        }

        /// Configures the leave notification embed thumbnail
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "thumbnail",
            category = "Management"
        )]
        async fn thumbnail(
            ctx: Context<'_>,
            #[description = "Embed thumbnail file upload"] thumbnail_file: Option<serenity::Attachment>,
            #[description = "Embed thumbnail web url"] thumbnail_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::thumbnail_impl(ctx, thumbnail_file, thumbnail_url).await
        }

        /// Configures the leave notification embed image
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            category = "Management"
        )]
        async fn image(
            ctx: Context<'_>,
            #[description = "Embed image file upload"] image_file: Option<serenity::Attachment>,
            #[description = "Embed image web url"] image_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::image_impl(ctx, image_file, image_url).await
        }

        /// Configures the leave notification embed author
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "author",
            category = "Management"
        )]
        async fn author(
            ctx: Context<'_>,
            #[description = "Embed author text"] author: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::author_impl(ctx, author).await
        }

        /// Configures the leave notification embed author icon
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "author-icon",
            category = "Management"
        )]
        async fn author_icon(
            ctx: Context<'_>,
            #[description = "Embed author icon file upload"] author_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed author icon web url"] author_icon_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::author_icon_impl(ctx, author_icon_file, author_icon_url).await
        }

        /// Configures the leave notification embed footer
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "footer",
            category = "Management"
        )]
        async fn footer(
            ctx: Context<'_>,
            #[description = "Embed footer text"] footer: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::footer_impl(ctx, footer).await
        }

        /// Configures the leave notification embed footer icon
        #[poise::command(
            slash_command,
            required_permissions = "ADMINISTRATOR",
            default_member_permissions = "ADMINISTRATOR",
            guild_only,
            rename = "footer-icon",
            category = "Management"
        )]
        async fn footer_icon(
            ctx: Context<'_>,
            #[description = "Embed footer icon file upload"] footer_icon_file: Option<
                serenity::Attachment,
            >,
            #[description = "Embed footer icon web url"] footer_icon_url: Option<String>,
        ) -> Result<(), Error> {
            record_ctx_fields!(ctx);
            CfgMemberLeave::footer_icon_impl(ctx, footer_icon_file, footer_icon_url).await
        }
    }
}

poise_instrument! {
    /// Tests the welcome functions by simulating a member joining the guild.
    #[poise::command(
        slash_command,
        prefix_command,
        owners_only,
        guild_only,
        hide_in_help,
        category = "Management"
    )]
    pub async fn test_member_add(ctx: Context<'_>) -> Result<(), Error> {
            record_ctx_fields!(ctx);
        ctx.defer_ephemeral().await?;
        let member = match ctx.author_member().await {
            Some(member) => member,
            None => return Err("Must be in guild".into()),
        };
        guild_member_add(ctx.serenity_context(), ctx.data(), &member).await?;
        ctx.send(
            CreateReply::default()
                .content("Acknowledged!")
                .ephemeral(true),
        )
        .await?;
        Ok(())
    }

    /// Tests the welcome functions by simulating a member leaving the guild.
    #[poise::command(
        slash_command,
        prefix_command,
        owners_only,
        guild_only,
        hide_in_help,
        category = "Management"
    )]
    pub async fn test_member_remove(ctx: Context<'_>) -> Result<(), Error> {
            record_ctx_fields!(ctx);
        ctx.defer_ephemeral().await?;
        let guild_id = require_guild_id(ctx)?;
        guild_member_remove(ctx.serenity_context(), ctx.data(), &guild_id, ctx.author()).await?;
        ctx.send(
            CreateReply::default()
                .content("Acknowledged!")
                .ephemeral(true),
        )
        .await?;
        Ok(())
    }
}
