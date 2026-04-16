use crate::core::orm::sea_orm_active_enums::TagType;
use crate::core::orm::{literatures, tags};
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use sqlx_core::types::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct FullData {
    pub id: i32,
    pub page_count: i32,
    pub rating: Decimal,
    pub upload: DateTimeWithTimeZone,
    pub literatures: Vec<InlineLiterature>,
    pub tags: Vec<InlineTag>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineLiterature {
    pub id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub lang: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineTag {
    pub id: i32,
    pub r#type: TagType,
    pub label: String,
    pub weight: i32,
}

impl From<literatures::Model> for InlineLiterature {
    fn from(l: literatures::Model) -> Self {
        InlineLiterature {
            id: l.id,
            title: l.title,
            description: l.description,
            lang: l.lang,
        }
    }
}

impl From<tags::Model> for InlineTag {
    fn from(t: tags::Model) -> InlineTag {
        InlineTag {
            id: t.id,
            r#type: t.r#type.into(),
            label: t.label,
            weight: Default::default(),
        }
    }
}

impl From<(tags::Model, i32)> for InlineTag {
    fn from((t, w): (tags::Model, i32)) -> InlineTag {
        InlineTag {
            id: t.id,
            r#type: t.r#type.into(),
            label: t.label,
            weight: w,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpireTime {
    Short,    // 20 mins
    Medium,   // 7 dats
    Long,     // 30 days
    Extended, // 90 days
    Permanent,
}

impl Default for ExpireTime {
    fn default() -> Self {
        ExpireTime::Long
    }
}

impl ExpireTime {
    pub fn to_duration(&self) -> Option<chrono::Duration> {
        match self {
            ExpireTime::Short => Some(chrono::Duration::minutes(20)),
            ExpireTime::Medium => Some(chrono::Duration::days(7)),
            ExpireTime::Long => Some(chrono::Duration::days(30)),
            ExpireTime::Extended => Some(chrono::Duration::days(90)),
            ExpireTime::Permanent => None,
        }
    }

    pub fn get_expire_time(
        &self,
        create_time: DateTimeWithTimeZone,
    ) -> Option<DateTimeWithTimeZone> {
        let Some(duration) = self.to_duration() else {
            return None;
        };

        Some(create_time + duration)
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
    pub literatures: Vec<Literature>, // 漫画数据的所有文字记录
    pub headers: HashMap<String, String>,
    pub source_site: String,
    pub extra: Option<serde_json::Value>, // 把crawler处理时需要的附加数据放这
}

impl Task {
    pub fn title(&self) -> &str {
        self.literatures
            .first()
            .and_then(|l| l.title.as_deref())
            .unwrap_or("Untitled")
    }
}
