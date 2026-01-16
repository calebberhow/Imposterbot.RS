use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(McServer::Table)
                    .col(string(McServer::GuildId))
                    .col(text(McServer::Name))
                    .col(text(McServer::Address))
                    .col(integer(McServer::Port))
                    .col(text(McServer::Version).default(""))
                    .col(text(McServer::Modpack).default(""))
                    .col(text(McServer::CustomDescription).default(""))
                    .col(text(McServer::Instructions).default(""))
                    .col(text(McServer::Thumbnail).default(""))
                    .primary_key(
                        IndexCreateStatement::new()
                            .col(McServer::GuildId)
                            .col(McServer::Name)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(WelcomeChannel::Table)
                    .col(string(WelcomeChannel::GuildId).primary_key())
                    .col(string(WelcomeChannel::ChannelId))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(WelcomeRoles::Table)
                    .col(string(WelcomeRoles::GuildId))
                    .col(string(WelcomeRoles::RoleId))
                    .primary_key(
                        IndexCreateStatement::new()
                            .col(WelcomeRoles::GuildId)
                            .col(WelcomeRoles::RoleId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(McServer::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WelcomeChannel::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WelcomeRoles::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum McServer {
    Table,
    GuildId,
    Name,
    Address,
    Port,
    Version,
    Modpack,
    CustomDescription,
    Instructions,
    Thumbnail,
}

#[derive(DeriveIden)]
pub enum WelcomeChannel {
    Table,
    GuildId,
    ChannelId,
}

#[derive(DeriveIden)]
enum WelcomeRoles {
    Table,
    GuildId,
    RoleId,
}
