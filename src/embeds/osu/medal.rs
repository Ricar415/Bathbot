use crate::{
    custom_client::OsekaiMedal,
    embeds::EmbedData,
    util::{
        constants::{FIELD_VALUE_SIZE, OSU_BASE},
        numbers::round,
    },
};

use cow_utils::CowUtils;
use std::fmt::Write;
use twilight_embed_builder::image_source::ImageSource;

pub struct MedalEmbed {
    url: String,
    thumbnail: ImageSource,
    title: String,
    fields: Vec<(String, String, bool)>,
}

impl MedalEmbed {
    pub fn new(medal: OsekaiMedal) -> Self {
        let mode = medal
            .mode
            .map_or_else(|| "Any".to_owned(), |mode| mode.to_string());
        let mods = medal
            .mods
            .map_or_else(|| "Any".to_owned(), |mods| mods.to_string());
        let mut fields = Vec::with_capacity(7);
        fields.push(("Description".to_owned(), medal.description, false));
        if let Some(solution) = medal.solution {
            fields.push(("Solution".to_owned(), solution, false));
        }
        fields.push(("Mode".to_owned(), mode, true));
        fields.push(("Mods".to_owned(), mods, true));
        fields.push(("Group".to_owned(), medal.group, true));
        if let Some(diff) = medal.difficulty {
            let diff_value = format!(
                "`Dedication: {}` • `Tapping: {}` • `Reading: {}` • `Patterns: {}`",
                round(diff.dedication),
                round(diff.tapping),
                round(diff.reading),
                round(diff.patterns),
            );
            fields.push(("Voted difficulty out of 10".to_owned(), diff_value, false));
        }

        if !medal.beatmaps.is_empty() {
            let len = medal.beatmaps.len();
            let mut map_value = String::with_capacity(256);
            for map in medal.beatmaps {
                let m = format!(
                    " - [{} [{}]]({}b/{})\n",
                    map.title, map.version, OSU_BASE, map.beatmap_id
                );
                if m.len() + map_value.len() + 7 >= FIELD_VALUE_SIZE {
                    map_value.push_str("`...`\n");
                    break;
                } else {
                    map_value += &m;
                }
            }
            map_value.pop();
            fields.push((format!("Beatmaps: {}", len), map_value, false));
        }
        if !medal.comments.is_empty() {
            let mut comment_value = String::with_capacity(256);
            let comment_iter = medal
                .comments
                .into_iter()
                .filter(|comment| comment.parent_id.is_none());
            for comment in comment_iter {
                let mut c =
                    String::with_capacity(16 + comment.content.len() + comment.username.len());
                c.push_str("```\n");
                c.push_str(comment.content.as_str());
                let _ = writeln!(c, "\n    - {} [{:+}]", comment.username, comment.vote_sum);
                c.push_str("```\n");
                if c.len() + comment_value.len() < FIELD_VALUE_SIZE {
                    comment_value += &c;
                }
            }
            comment_value.pop();
            if !comment_value.is_empty() {
                fields.push(("Comments".to_owned(), comment_value, false));
            }
        }
        let title = medal.name;
        let thumbnail = ImageSource::url(medal.url).unwrap();
        let url = format!(
            "https://osekai.net/medals/?medal={}",
            title.cow_replace(' ', "+").cow_replace(',', "%2C")
        );
        Self {
            url,
            thumbnail,
            title,
            fields,
        }
    }
}

impl EmbedData for MedalEmbed {
    fn title(&self) -> Option<&str> {
        Some(&self.title)
    }
    fn url(&self) -> Option<&str> {
        Some(&self.url)
    }
    fn thumbnail(&self) -> Option<&ImageSource> {
        Some(&self.thumbnail)
    }
    fn fields(&self) -> Option<Vec<(String, String, bool)>> {
        Some(self.fields.clone())
    }
}