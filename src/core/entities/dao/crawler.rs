use crate::core::entities::orm::sea_orm_active_enums::TagType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
