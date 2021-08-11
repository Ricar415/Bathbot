use super::RecentArgs;
use crate::{
    embeds::{EmbedData, RecentEmbed},
    tracking::process_tracking,
    util::{
        constants::{GENERAL_ISSUE, OSU_API_ISSUE},
        MessageExt,
    },
    BotResult, CommandData, Context, MessageBuilder,
};

use rosu_v2::prelude::{
    GameMode, Grade, OsuError,
    RankStatus::{Approved, Loved, Qualified, Ranked},
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub(super) async fn _recent(
    ctx: Arc<Context>,
    data: CommandData<'_>,
    args: RecentArgs,
) -> BotResult<()> {
    // TODO: Check for -p?
    // if args.has_dash_p {
    //     let prefix = ctx.config_first_prefix(msg.guild_id);

    //     let content = format!(
    //         "`{prefix}recent{mode} -p`? \
    //         Try putting the number right after the command, e.g. `{prefix}recent{mode}42`.\n\
    //         Alternatively you can checkout the `recentpages{mode}` command.",
    //         mode = match mode {
    //             GameMode::STD => "",
    //             GameMode::MNA => "mania",
    //             GameMode::TKO => "taiko",
    //             GameMode::CTB => "ctb",
    //         },
    //         prefix = prefix
    //     );

    //     return data.error(&ctx, content).await;
    // }

    let name = match args.name {
        Some(name) => name,
        None => match ctx.get_link(data.author()?.id.0) {
            Some(name) => name,
            None => return super::require_link(&ctx, &data).await,
        },
    };

    let mode = args.mode;

    // Retrieve the user and their recent scores
    let user_fut = super::request_user(&ctx, &name, Some(mode));

    let scores_fut = ctx
        .osu()
        .user_scores(name.as_str())
        .recent()
        .mode(mode)
        .limit(50)
        .include_fails(true);

    let (user, mut scores) = match tokio::try_join!(user_fut, scores_fut) {
        Ok((_, scores)) if scores.is_empty() => {
            let content = format!(
                "No recent {}plays found for user `{}`",
                match mode {
                    GameMode::STD => "",
                    GameMode::TKO => "taiko ",
                    GameMode::CTB => "ctb ",
                    GameMode::MNA => "mania ",
                },
                name,
            );

            return data.error(&ctx, content).await;
        }
        Ok((user, scores)) => (user, scores),
        Err(OsuError::NotFound) => {
            let content = format!("User `{}` was not found", name);

            return data.error(&ctx, content).await;
        }
        Err(why) => {
            let _ = data.error(&ctx, OSU_API_ISSUE).await;

            return Err(why.into());
        }
    };

    let num = args.index.unwrap_or(1).saturating_sub(1);
    let mut iter = scores.iter_mut().skip(num);

    let (score, tries) = match iter.next() {
        Some(score) => match super::prepare_score(&ctx, score).await {
            Ok(_) => {
                let mods = score.mods;
                let map_id = map_id!(score).unwrap();

                let tries = 1 + iter
                    .take_while(|s| map_id!(s).unwrap() == map_id && s.mods == mods)
                    .count();

                (score, tries)
            }
            Err(why) => {
                let _ = data.error(&ctx, OSU_API_ISSUE).await;

                return Err(why.into());
            }
        },
        None => {
            let content = format!(
                "There {verb} only {num} score{plural} in `{name}`'{genitive} recent history.",
                verb = if scores.len() != 1 { "are" } else { "is" },
                num = scores.len(),
                plural = if scores.len() != 1 { "s" } else { "" },
                name = name,
                genitive = if name.ends_with('s') { "" } else { "s" }
            );

            return data.error(&ctx, content).await;
        }
    };

    let map = score.map.as_ref().unwrap();

    // Prepare retrieval of the the user's top 50 and score position on the map
    let map_score_fut = async {
        if score.grade != Grade::F && matches!(map.status, Ranked | Loved | Qualified | Approved) {
            let fut = ctx
                .osu()
                .beatmap_user_score(map.map_id, user.user_id)
                .mode(mode);

            Some(fut.await)
        } else {
            None
        }
    };

    let best_fut = async {
        if score.grade != Grade::F && map.status == Ranked {
            let fut = ctx
                .osu()
                .user_scores(user.user_id)
                .best()
                .limit(100)
                .mode(mode);

            Some(fut.await)
        } else {
            None
        }
    };

    // Retrieve and parse response
    let (map_score_result, best_result) = tokio::join!(map_score_fut, best_fut);

    let map_score = match map_score_result {
        None | Some(Err(OsuError::NotFound)) => None,
        Some(Ok(score)) => Some(score),
        Some(Err(why)) => {
            unwind_error!(warn, why, "Error while getting global scores: {}");

            None
        }
    };

    let mut best = match best_result {
        None => None,
        Some(Ok(scores)) => Some(scores),
        Some(Err(why)) => {
            unwind_error!(warn, why, "Error while getting top scores: {}");

            None
        }
    };

    let data_fut = RecentEmbed::new(&user, score, best.as_deref(), map_score.as_ref(), false);

    let embed_data = match data_fut.await {
        Ok(data) => data,
        Err(why) => {
            let _ = data.error(&ctx, GENERAL_ISSUE).await;

            return Err(why);
        }
    };

    // Creating the embed
    let content = format!("Try #{}", tries);
    let embed = embed_data.as_builder().build();
    let builder = MessageBuilder::new().content(content).embed(embed);
    let response = data.create_message(&ctx, builder).await?;

    // TODO
    // ctx.store_msg(response.id);

    // Set map on garbage collection list if unranked
    let gb = ctx.map_garbage_collector(map);

    // * Note: Don't store maps in DB as their max combo isn't available

    // Process user and their top scores for tracking
    if let Some(ref mut scores) = best {
        if let Err(why) = ctx.psql().store_scores_maps(scores.iter()).await {
            unwind_error!(warn, why, "Error while storing best maps in DB: {}");
        }

        process_tracking(&ctx, mode, scores, Some(&user)).await;
    }

    let data = data.compact();

    // Wait for minimizing
    tokio::spawn(async move {
        gb.execute(&ctx).await;
        sleep(Duration::from_secs(45)).await;

        // TODO
        // if !ctx.remove_msg(response.id) {
        //     return;
        // }

        let embed = embed_data.into_builder().build();
        let builder = MessageBuilder::new().embed(embed);

        if let Err(why) = data.update_message(&ctx, builder, response).await {
            unwind_error!(warn, why, "Error minimizing recent msg: {}");
        }
    });

    Ok(())
}

#[command]
#[short_desc("Display a user's most recent play")]
#[long_desc(
    "Display a user's most recent play.\n\
    To get a previous recent score, you can add a number right after the command,\n\
    e.g. `r42 badewanne3` to get the 42nd most recent score."
)]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("r", "rs")]
pub async fn recent(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentArgs::args(&ctx, &mut args, GameMode::STD, num) {
                Ok(recent_args) => {
                    _recent(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}

#[command]
#[short_desc("Display a user's most recent mania play")]
#[long_desc(
    "Display a user's most recent play.\n\
    To get a previous recent score, you can add a number right after the command,\n\
    e.g. `rm42 badewanne3` to get the 42nd most recent score."
)]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rm")]
pub async fn recentmania(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentArgs::args(&ctx, &mut args, GameMode::MNA, num) {
                Ok(recent_args) => {
                    _recent(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}

#[command]
#[short_desc("Display a user's most recent taiko play")]
#[long_desc(
    "Display a user's most recent play.\n\
    To get a previous recent score, you can add a number right after the command,\n\
    e.g. `rt42 badewanne3` to get the 42nd most recent score."
)]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rt")]
pub async fn recenttaiko(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentArgs::args(&ctx, &mut args, GameMode::TKO, num) {
                Ok(recent_args) => {
                    _recent(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}

#[command]
#[short_desc("Display a user's most recent ctb play")]
#[long_desc(
    "Display a user's most recent play.\n\
    To get a previous recent score, you can add a number right after the command,\n\
    e.g. `rc42 badewanne3` to get the 42nd most recent score."
)]
#[usage("[username]")]
#[example("badewanne3")]
#[aliases("rc")]
pub async fn recentctb(ctx: Arc<Context>, data: CommandData) -> BotResult<()> {
    match data {
        CommandData::Message { msg, mut args, num } => {
            match RecentArgs::args(&ctx, &mut args, GameMode::CTB, num) {
                Ok(recent_args) => {
                    _recent(ctx, CommandData::Message { msg, args, num }, recent_args).await
                }
                Err(content) => msg.error(&ctx, content).await,
            }
        }
        CommandData::Interaction { command } => super::slash_recent(ctx, command).await,
    }
}