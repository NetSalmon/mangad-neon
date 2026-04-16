use std::collections::HashMap;
use crate::core::orm::sea_orm_active_enums::TagType;
use crate::core::orm::{literatures, tags};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use sqlx_core::types::Decimal;

#[derive(Debug, Clone, Serialize)]
pub struct ApiResp<T> {
    pub ok: bool,
    #[serde(skip_serializing)]
    pub status_code: Option<StatusCode>,
    pub result: T,
}

impl<T: Serialize> IntoResponse for ApiResp<T> {
    fn into_response(self) -> Response {
        let code = self.status_code.unwrap_or(StatusCode::OK);
        (code, Json(self)).into_response()
    }
}

impl<T> ApiResp<T> {
    pub fn success(result: T) -> ApiResp<T> {
        ApiResp {
            ok: true,
            status_code: Some(StatusCode::OK),
            result,
        }
    }

    pub fn failed(result: T) -> ApiResp<T> {
        ApiResp {
            ok: false,
            status_code: None,
            result,
        }
    }

    pub fn success_with(code: StatusCode, result: T) -> ApiResp<T> {
        ApiResp {
            ok: false,
            status_code: Some(code),
            result,
        }
    }

    pub fn failed_with(code: StatusCode, result: T) -> ApiResp<T> {
        ApiResp {
            ok: false,
            status_code: Some(code),
            result,
        }
    }
}

impl From<&str> for ApiResp<String> {
    fn from(t: &str) -> ApiResp<String> {
        ApiResp {
            ok: true,
            status_code: None,
            result: t.to_string(),
        }
    }
}

impl<T: Serialize> From<T> for ApiResp<T> {
    fn from(t: T) -> ApiResp<T> {
        ApiResp {
            ok: true,
            status_code: None,
            result: t,
        }
    }
}

impl<T: Serialize> From<(StatusCode, T)> for ApiResp<T> {
    fn from(t: (StatusCode, T)) -> ApiResp<T> {
        let is_ok = !t.0.is_client_error() && !t.0.is_server_error();
        ApiResp {
            ok: is_ok,
            status_code: Some(t.0),
            result: t.1,
        }
    }
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub filter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubTaskResult {
    pub tid: i32,
    pub index: i32,
    pub status: SubTaskStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SubTaskStatus {
    Ok,
    Err(String),
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