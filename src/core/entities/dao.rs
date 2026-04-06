use crate::core::entities::dao::active::TagType;
use crate::core::entities::orm::{literatures, sea_orm_active_enums, tags};
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

pub mod active;
pub mod crawler;

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
    pub ref_count: i32,
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
            ref_count: t.ref_count,
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
                sea_orm_active_enums::TagType::Genre => &mut genres,
                sea_orm_active_enums::TagType::Artist => &mut artists,
                sea_orm_active_enums::TagType::Origin => &mut origins,
                sea_orm_active_enums::TagType::Serial => &mut serials,
                sea_orm_active_enums::TagType::Chara => &mut characters,
                sea_orm_active_enums::TagType::Lang => &mut languages,
                sea_orm_active_enums::TagType::Group => &mut groups,
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