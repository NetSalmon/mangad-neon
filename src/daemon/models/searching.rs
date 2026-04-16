use mangad_neon::core::orm::sea_orm_active_enums::TagType;
use mangad_neon::core::orm::{literatures, tags};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    id: i32,
    title: Option<String>,
    description: Option<String>,
    lang: String,
    genres: Vec<String>,
    artists: Vec<String>,
    origins: Vec<String>,
    serials: Vec<String>,
    characters: Vec<String>,
    groups: Vec<String>,
    languages: Vec<String>,
}

impl From<(literatures::Model, Vec<tags::Model>)> for Document {
    fn from((l, tags): (literatures::Model, Vec<tags::Model>)) -> Self {
        let mut genres = vec![];
        let mut artists = vec![];
        let mut origins = vec![];
        let mut serials = vec![];
        let mut characters = vec![];
        let mut groups = vec![];
        let mut languages = vec![];

        for tag in tags {
            let list = match tag.r#type {
                TagType::Genre => &mut genres,
                TagType::Artist => &mut artists,
                TagType::Origin => &mut origins,
                TagType::Serial => &mut serials,
                TagType::Character => &mut characters,
                TagType::Lang => &mut languages,
                TagType::Group => &mut groups,
            };

            list.push(tag.label.to_string());
        }

        Self {
            id: l.id,
            title: l.title,
            description: l.description,
            lang: l.lang,
            genres,
            artists,
            origins,
            serials,
            characters,
            groups,
            languages,
        }
    }
}
