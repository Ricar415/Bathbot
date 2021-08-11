use super::{ErrorType, RecentListArgs};
use crate::{
    embeds::{EmbedData, RecentListEmbed},
    pagination::{Pagination, RecentListPagination},
    util::{
        constants::{GENERAL_ISSUE, OSU_API_ISSUE},
        numbers, MessageExt,
    },
    BotResult, CommandData, Context,
};

use futures::future::TryFutureExt;
use rosu_v2::prelude::{GameMode, OsuError};
use std::sync::Arc;

pub(super) async fn _recentlist(
    ctx: Arc<Context>,
    data: CommandData<'_>,
    args: RecentListArgs,
) -> BotResult<()> {
    let name = match args.name {
        Some(name) => name,
        None => match ctx.get_link(data.author()?.id.0) {
            Some(name) => name,
            None => return super::require_link(&ctx, &data).await,
        },
    };

    let mode = args.mode;

    // Retrieve the user and their recent scores
    let user_fut = super::request_user(&ctx, &name, Some(mode)).map_err(From::from);

    let scores_fut = ctx
        .osu()
        .user_scores(name.as_str())
        .recent()
        .mode(mode)
        .limit(100)
        .include_fails(true);

    let scores_fut = super::prepare_scores(&ctx, scores_fut);

    let (user, scores) = match tokio::try_join!(user_fut, scores_fut) {
        Ok((_, scores)) if scores.is_empty() => {
            let content = format!(
                "No recent {}plays found for user `{}`",
                match mode {
                    GameMode::STD => "",
                    GameMode::TKO => "taiko ",
                    GameMode::CTB => "ctb ",
                    GameMode::MNA => "mania ",
                },
                name
            );

            return data.error(&ctx, content).await;
        }
        Ok((user, scores)) => (user, scores),
        Err(ErrorType::Osu(OsuError::NotFound)) => {
            let content = format!("User `{}` was not found", name);

            return data.error(&ctx, content).await;
        }
        Err(ErrorType::Osu(why)) => {
            let _ = data.error(&ctx, OSU_API_ISSUE).await;

            return Err(why.into());
        }
        Err(ErrorType::Bot(why)) => {
            let _ = data.error(&ctx, GENERAL_ISSUE).await;

            return Err(why);
        }
    };

    let pages = numbers::div_euclid(10, scores.len());
    let scores_iter = scores.iter().take(10);

    let embed = match RecentListEmbed::new(&user, scores_iter, (1, pages)).await {
        Ok(data) => data.into_builder().build(),
        Err(why) => {
            let _ = data.error(&ctx, GENERAL_ISSUE).await;

            return Err(why);
        }
    };

    // Creating the embed
    let builder = embed.into();
    let response_raw = data.create_message(&ctx, builder).await?;

    // Skip pagination if too few entries
    if scores.len() <= 10 {
        return Ok(());
    }

    let response = data.get_response(&ctx, response_raw).await?;

    // Pagination
    let pagination = RecentListPagination::new(Arc::clone(&ctx), response, user, scores);
    let owner = data.author()?.id;

    tokio::spawn(async move {
        if let Err(why) = pagination.start(&ctx, owner, 60).await {
            unwind_error!(warn, why, "Pagination error (recentlist): {}")
        }
    });

    Ok(())
}

#[command]
#[short_desc("Display a list of a user's most recent plays")]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rl")]
pub async fn recentlist(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentListArgs::args(&ctx, &mut args, GameMode::STD) {
                Ok(recent_args) => {
                    _recentlist(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}

#[command]
#[short_desc("Display a list of a user's most recent mania plays")]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rlm")]
pub async fn recentlistmania(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentListArgs::args(&ctx, &mut args, GameMode::MNA) {
                Ok(recent_args) => {
                    _recentlist(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}

#[command]
#[short_desc("Display a list of a user's most recent taiko plays")]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rlt")]
pub async fn recentlisttaiko(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentListArgs::args(&ctx, &mut args, GameMode::TKO) {
                Ok(recent_args) => {
                    _recentlist(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}

#[command]
#[short_desc("Display a list of a user's most recent ctb plays")]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rlc")]
pub async fn recentlistctb(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentListArgs::args(&ctx, &mut args, GameMode::CTB) {
                Ok(recent_args) => {
                    _recentlist(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}