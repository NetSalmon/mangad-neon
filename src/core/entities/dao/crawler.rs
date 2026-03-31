use crate::core::entities::orm::sea_orm_active_enums::TagType as ORMTagType;
use crate::error::Error;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use url::Url;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TagType {
    Genre,
    Artist,
    Origin,
    Serial,
    Chara,
    Lang,
    Group,
}

impl TagType {
    pub fn to_orm(&self) -> ORMTagType {
        match self {
            TagType::Genre => ORMTagType::Genre,
            TagType::Artist => ORMTagType::Artist,
            TagType::Origin => ORMTagType::Origin,
            TagType::Serial => ORMTagType::Serial,
            TagType::Chara => ORMTagType::Chara,
            TagType::Lang => ORMTagType::Lang,
            TagType::Group => ORMTagType::Group,
        }
    }

    pub fn from_orm(value: ORMTagType) -> TagType {
        match value {
            ORMTagType::Genre => TagType::Genre,
            ORMTagType::Artist => TagType::Artist,
            ORMTagType::Origin => TagType::Origin,
            ORMTagType::Serial => TagType::Serial,
            ORMTagType::Chara => TagType::Chara,
            ORMTagType::Lang => TagType::Lang,
            ORMTagType::Group => TagType::Group,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    pub label: String,
    pub r#type: TagType,
    pub canonical_id: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Literature {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Task {
    pub images: Vec<String>,
    pub tags: Vec<Tag>,
    pub literatures: Vec<Literature>,
    pub headers: HashMap<String, String>,
    pub source_site: String,
    pub source_id: String,
}

#[derive(Clone)]
pub struct SubTask {
    pub url: Url,
    pub headers: HeaderMap,
    pub source_site: Arc<String>,
    pub source_id: Arc<String>,
    pub index: i32,
}

impl Task {
    pub fn spilt(&self) -> Result<Vec<SubTask>, Error> {
        let mut subtasks = vec![];

        let mut headers = HeaderMap::new();
        for (k, v) in self.headers.iter() {
            headers.insert(HeaderName::from_str(k)?, HeaderValue::from_str(v)?);
        }

        let source_site = Arc::new(self.source_site.clone());
        let source_id = Arc::new(self.source_id.clone());

        for (index, image) in self.images.iter().enumerate() {
            let subtask = SubTask {
                url: image.parse()?,
                headers: headers.clone(),
                source_site: source_site.clone(),
                source_id: source_id.clone(),
                index: index as i32,
            };

            subtasks.push(subtask);
        }

        Ok(subtasks)
    }
}
