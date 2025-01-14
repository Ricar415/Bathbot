use std::fmt::Write;

use bathbot_cache::model::CachedArchive;
use bathbot_macros::EmbedData;
use bathbot_model::twilight_model::guild::Guild;
use bathbot_psql::model::configs::{
    GuildConfig, HideSolutions, ListSize, MinimizedPp, Retries, ScoreSize,
};
use bathbot_util::AuthorBuilder;
use twilight_model::channel::message::embed::EmbedField;

use super::config::create_field;

#[derive(EmbedData)]
pub struct ServerConfigEmbed {
    author: AuthorBuilder,
    description: String,
    fields: Vec<EmbedField>,
    footer: &'static str,
    title: &'static str,
}

impl ServerConfigEmbed {
    pub fn new(guild: CachedArchive<Guild>, config: GuildConfig, authorities: &[String]) -> Self {
        let mut author = AuthorBuilder::new(guild.name.as_ref());

        if let Some(hash) = guild.icon.as_ref() {
            let url = format!(
                "https://cdn.discordapp.com/icons/{}/{hash}.{}",
                guild.id,
                if hash.animated { "gif" } else { "webp" }
            );

            author = author.icon_url(url);
        }

        let title = "Current server configuration:";

        let mut description = String::with_capacity(256);

        description.push_str("```\nAuthorities: ");

        let mut authorities = authorities.iter();

        if let Some(auth) = authorities.next() {
            let _ = write!(description, "@{auth}");

            for auth in authorities {
                let _ = write!(description, ", @{auth}");
            }
        } else {
            description.push_str("None");
        }

        description.push_str("\nPrefixes: ");
        let mut prefixes = config.prefixes.iter();

        if let Some(prefix) = prefixes.next() {
            let _ = write!(description, "`{prefix}`");

            for prefix in prefixes {
                let _ = write!(description, ", `{prefix}`");
            }
        }

        let track_limit = config.track_limit.unwrap_or(50);
        let _ = writeln!(description, "\nDefault track limit: {track_limit}\n```");

        let fields = vec![
            create_field(
                "Render button",
                config.render_button.unwrap_or(true),
                &[(false, "hide"), (true, "let user decide")],
            ),
            create_field(
                "Song commands",
                config.allow_songs.unwrap_or(true),
                &[(true, "enabled"), (false, "disabled")],
            ),
            create_field(
                "Allow custom skins",
                config.allow_custom_skins.unwrap_or(true),
                &[(true, "true"), (false, "false")],
            ),
            create_field(
                "Score embeds*",
                config.score_size.unwrap_or_default(),
                &[
                    (ScoreSize::AlwaysMinimized, "always minimized"),
                    (ScoreSize::AlwaysMaximized, "always maximized"),
                    (ScoreSize::InitialMaximized, "initial maximized"),
                ],
            ),
            create_field(
                "List embeds*",
                config.list_size.unwrap_or_default(),
                &[
                    (ListSize::Condensed, "condensed"),
                    (ListSize::Detailed, "detailed"),
                    (ListSize::Single, "single"),
                ],
            ),
            create_field(
                "Medal solutions",
                config.hide_medal_solution.unwrap_or(HideSolutions::ShowAll),
                &[
                    (HideSolutions::ShowAll, "show"),
                    (HideSolutions::HideHushHush, "hide hush-hush"),
                    (HideSolutions::HideAll, "hide all"),
                ],
            ),
            create_field(
                "Minimized PP*",
                config.minimized_pp.unwrap_or_default(),
                &[(MinimizedPp::MaxPp, "max pp"), (MinimizedPp::IfFc, "if FC")],
            ),
            create_field(
                "Retries*",
                config.retries.unwrap_or(Retries::ConsiderMods),
                &[
                    (Retries::Hide, "hide"),
                    (Retries::ConsiderMods, "reset on different mods"),
                    (Retries::IgnoreMods, "ignore mods"),
                ],
            ),
        ];

        Self {
            author,
            description,
            fields,
            footer: "*: Only applies if not set in the member's user config",
            title,
        }
    }
}
