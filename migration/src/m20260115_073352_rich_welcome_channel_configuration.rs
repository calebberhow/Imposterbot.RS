use sea_orm_migration::{prelude::*, schema::*};

use crate::m20220101_000001_initial;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MemberNotificationMessage::Table)
                    .col(string(MemberNotificationMessage::GuildId).not_null())
                    .col(boolean(MemberNotificationMessage::Join).not_null())
                    .col(
                        text(MemberNotificationMessage::Content)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        text(MemberNotificationMessage::Description)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        boolean(MemberNotificationMessage::ThumbnailIsFile)
                            .not_null()
                            .default(false),
                    )
                    .col(
                        text(MemberNotificationMessage::ThumbnailUrl)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        text(MemberNotificationMessage::Author)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        boolean(MemberNotificationMessage::AuthorIconIsFile)
                            .not_null()
                            .default(false),
                    )
                    .col(
                        text(MemberNotificationMessage::AuthorIconUrl)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        text(MemberNotificationMessage::Footer)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        boolean(MemberNotificationMessage::FooterIconIsFile)
                            .not_null()
                            .default(false),
                    )
                    .col(
                        text(MemberNotificationMessage::FooterIconUrl)
                            .not_null()
                            .default(""),
                    )
                    .primary_key(
                        IndexCreateStatement::new()
                            .col(MemberNotificationMessage::GuildId)
                            .col(MemberNotificationMessage::Join)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(m20220101_000001_initial::WelcomeChannel::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MemberNotificationChannel::Table)
                    .col(string(MemberNotificationChannel::GuildId).not_null())
                    .col(boolean(MemberNotificationChannel::Join).not_null())
                    .col(string(MemberNotificationChannel::ChannelId).not_null())
                    .primary_key(
                        IndexCreateStatement::new()
                            .col(MemberNotificationChannel::GuildId)
                            .col(MemberNotificationChannel::Join)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .table(MemberNotificationChannel::Table)
                    .name("idx-member-notification-channel-guild-join")
                    .col(MemberNotificationChannel::GuildId)
                    .col(MemberNotificationChannel::Join)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MemberNotificationMessage::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(MemberNotificationChannel::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(m20220101_000001_initial::WelcomeChannel::Table)
                    .col(string(m20220101_000001_initial::WelcomeChannel::GuildId).primary_key())
                    .col(string(m20220101_000001_initial::WelcomeChannel::ChannelId))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum MemberNotificationMessage {
    Table,
    GuildId, // Primary Key
    Join,    // Primary Key
    Content,
    Description,
    ThumbnailIsFile,
    ThumbnailUrl,
    Author,
    AuthorIconIsFile,
    AuthorIconUrl,
    Footer,
    FooterIconIsFile,
    FooterIconUrl,
}

#[derive(DeriveIden)]
enum MemberNotificationChannel {
    Table,
    GuildId, // Primary Key
    Join,    // Primary Key
    #[allow(unused)]
    ChannelId,
}
