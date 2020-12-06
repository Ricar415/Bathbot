use crate::{
    custom_client::SnipeRecent,
    embeds::{osu, Author, EmbedData},
    util::constants::AVATAR_URL,
};

use rosu::model::User;
use std::collections::HashMap;
use twilight_embed_builder::image_source::ImageSource;

pub struct SnipedEmbed {
    description: Option<String>,
    thumbnail: ImageSource,
    title: &'static str,
    author: Author,
    image: Option<ImageSource>,
    fields: Option<Vec<(String, String, bool)>>,
}

impl SnipedEmbed {
    pub fn new(user: User, sniper: Vec<SnipeRecent>, snipee: Vec<SnipeRecent>) -> Self {
        let thumbnail = ImageSource::url(format!("{}{}", AVATAR_URL, user.user_id)).unwrap();
        let author = osu::get_user_author(&user);
        let title = "National snipe scores of the last 8 weeks";

        if sniper.is_empty() && snipee.is_empty() {
            let description = format!(
                "`{}` was neither sniped nor sniped other people",
                user.username
            );
            return Self {
                description: Some(description),
                thumbnail,
                author,
                title,
                image: None,
                fields: None,
            };
        }

        let mut fields = Vec::with_capacity(2);

        if !sniper.is_empty() {
            let mut victims = HashMap::new();
            for score in sniper.iter() {
                *victims.entry(score.sniped.as_deref().unwrap()).or_insert(0) += 1;
            }
            let (most_name, most_count) = victims.iter().max_by_key(|(_, count)| *count).unwrap();

            let name = format!("Sniped by {}:", user.username);
            let value = format!(
                "Total count: {}\n\
                Different victims: {}\n\
                Targeted the most: {} ({})",
                sniper.len(),
                victims.len(),
                most_name,
                most_count
            );
            fields.push((name, value, false));
        }

        if !snipee.is_empty() {
            let mut snipers = HashMap::new();
            for score in snipee.iter() {
                *snipers.entry(score.sniper.as_str()).or_insert(0) += 1;
            }
            let (most_name, most_count) = snipers.iter().max_by_key(|(_, count)| *count).unwrap();

            let name = format!("Sniped {}:", user.username);
            let value = format!(
                "Total count: {}\n\
                Different snipers: {}\n\
                Targeted the most: {} ({})",
                snipee.len(),
                snipers.len(),
                most_name,
                most_count
            );
            fields.push((name, value, false));
        }

        Self {
            title,
            author,
            thumbnail,
            description: None,
            fields: Some(fields),
            image: Some(ImageSource::attachment("sniped_graph.png").unwrap()),
        }
    }
}

impl EmbedData for SnipedEmbed {
    fn title(&self) -> Option<&str> {
        Some(self.title)
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn thumbnail(&self) -> Option<&ImageSource> {
        Some(&self.thumbnail)
    }
    fn image(&self) -> Option<&ImageSource> {
        self.image.as_ref()
    }
    fn author(&self) -> Option<&Author> {
        Some(&self.author)
    }
    fn fields(&self) -> Option<Vec<(String, String, bool)>> {
        self.fields.clone()
    }
}