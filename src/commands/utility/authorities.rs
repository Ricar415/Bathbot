use crate::{
    bail,
    util::{constants::GENERAL_ISSUE, content_safe, matcher, MessageExt},
    Args, BotResult, Context,
};

use regex::Regex;
use std::{collections::hash_map::Entry, fmt::Write, str::FromStr, sync::Arc};
use twilight::model::{
    channel::Message,
    guild::Permissions,
    id::{GuildId, RoleId},
};

#[command]
#[only_guilds()]
#[authority()]
#[short_desc("Adjust authority roles for a guild")]
#[long_desc(
    "Decide which roles should be considered authority roles. \n\
    Authority roles enable the usage of certain commands like \
    `addstream` or `prune`.\n\
    Roles can be given as mention or as role id (up to 10 roles possible).\n\
    If you want to see the current authority roles, just pass \
    `-show` as argument"
)]
#[usage("[@role1] [id of role2] ...")]
#[example("-show")]
#[example("@Moderator @Mod 83794728403223 @BotCommander")]
#[aliases("authority")]
async fn authorities(ctx: Arc<Context>, msg: &Message, args: Args) -> BotResult<()> {
    let guild_id = msg.guild_id.unwrap();
    let args = args.take_n(10);

    // Check if the user just wants to see the current authorities
    match args.current().unwrap_or_default() {
        "-show" | "show" => {
            let roles = ctx.config_authorities(guild_id);
            let mut content = "Current authority roles for this server: ".to_owned();
            role_string(&ctx, &roles, guild_id, &mut content);

            // Send the message
            msg.respond(&ctx, content).await?;
            return Ok(());
        }
        _ => {}
    }

    // Make sure arguments are roles of the guild
    let mut new_auths = Vec::with_capacity(10);
    for arg in args {
        let role_id = match matcher::get_mention_role(arg) {
            Some(id) => id,
            None => {
                let content = format!("Expected role mention or role id, got `{}`", arg);
                msg.respond(&ctx, content).await?;
                return Ok(());
            }
        };
        match ctx.cache.get_role(RoleId(role_id), guild_id) {
            Some(role) => new_auths.push(role),
            None => {
                let content = format!("No role with id {} found in this guild", role_id);
                msg.respond(&ctx, content).await?;
                return Ok(());
            }
        }
    }

    // Make sure the author is still an authority after applying new roles
    let mut member_roles = match ctx.cache.get_member(msg.author.id, guild_id) {
        Some(member) => member.roles.clone(),
        None => {
            msg.respond(&ctx, GENERAL_ISSUE).await?;
            bail!("Member {} not cached for guild {}", msg.author.id, guild_id);
        }
    };
    member_roles.retain(|role| new_auths.iter().any(|new| &new.id == role));
    if !is_auth_with_roles(&ctx, &member_roles, guild_id) {
        let content = "You cannot set authority roles to something \
                that would make you lose authority status.";
        msg.respond(&ctx, content).await?;
        return Ok(());
    }

    ctx.update_config(guild_id, move |config| {
        config.authorities = new_auths.into_iter().map(|role| role.id.0).collect();
    });

    // Send the message
    let mut content = "Successfully changed the authority roles to: ".to_owned();
    let roles = ctx.config_authorities(guild_id);
    role_string(&ctx, &roles, guild_id, &mut content);
    msg.respond(&ctx, content).await?;
    Ok(())
}

fn role_string(ctx: &Context, roles: &[u64], guild_id: GuildId, content: &mut String) {
    if roles.is_empty() {
        content.push_str("None");
    } else {
        content.reserve(roles.len() * 20);
        let mut iter = roles.iter();
        let _ = write!(content, "`@&{}`", iter.next().unwrap());
        for role in iter {
            let _ = write!(content, ", `@&{}`", role);
        }
        content_safe(&ctx, content, Some(guild_id));
    }
}

fn is_auth_with_roles(ctx: &Context, role_ids: &[RoleId], guild_id: GuildId) -> bool {
    for &role_id in role_ids {
        if let Some(role) = ctx.cache.get_role(role_id, guild_id) {
            if role.permissions.contains(Permissions::ADMINISTRATOR) {
                return true;
            }
        }
    }
    false
}