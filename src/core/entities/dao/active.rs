use crate::core::entities::orm::{
    literatures, metadata, sea_orm_active_enums, tag_metadata, tags, tasks, tokens,
};
use sea_orm::ActiveModelTrait;
use sea_orm::prelude::{DateTimeWithTimeZone, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait IntoActiveModel {
    type ActiveModel: ActiveModelTrait;
    fn into_active_model(self) -> Self::ActiveModel;
}

macro_rules! set {
    ($from:ident => $to:path { $( $field:ident $(: enum $enum_path:path)? ),* $(,)? } ) => {
        impl IntoActiveModel for $from {
            type ActiveModel = $to;
            fn into_active_model(self) -> Self::ActiveModel {
                Self::ActiveModel {
                    $(
                        $field: set!(@field self, $field $(, $enum_path)?),
                    )*
                }
            }
        }
    };

    (@field $self:ident, $field:ident) => {
        if let Some(v) = $self.$field {
            sea_orm::Set(v)
        } else {
            sea_orm::ActiveValue::NotSet
        }
    };

    (@field $self:ident, $field:ident, $enum_path:path) => {
        if let Some(v) = $self.$field {
            sea_orm::Set(v.into())
        } else {
            sea_orm::ActiveValue::NotSet
        }
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Literatures {
    pub id: Option<i32>,
    pub metadata_id: Option<i32>,
    pub title: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub lang: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub id: Option<i32>,
    pub page_count: Option<i32>,
    pub upload: Option<DateTimeWithTimeZone>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagMetadata {
    pub metadata_id: Option<i32>,
    pub tag_id: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tags {
    pub id: Option<i32>,
    pub r#type: Option<TagType>,
    pub label: Option<String>,
    pub canonical_id: Option<Option<i32>>,
    pub ref_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Success,
    Processing,
    Failure,
}

impl From<TaskStatus> for sea_orm_active_enums::TaskStatus {
    fn from(task_status: TaskStatus) -> Self {
        match task_status {
            TaskStatus::Success => sea_orm_active_enums::TaskStatus::Success,
            TaskStatus::Processing => sea_orm_active_enums::TaskStatus::Processing,
            TaskStatus::Failure => sea_orm_active_enums::TaskStatus::Failure,
        }
    }
}

impl From<sea_orm_active_enums::TaskStatus> for TaskStatus {
    fn from(t: sea_orm_active_enums::TaskStatus) -> Self {
        match t {
            sea_orm_active_enums::TaskStatus::Success    => TaskStatus::Success   ,
            sea_orm_active_enums::TaskStatus::Processing => TaskStatus::Processing,
            sea_orm_active_enums::TaskStatus::Failure    => TaskStatus::Failure   ,
        }
    }
}

impl From<sea_orm_active_enums::TagType> for TagType {
    fn from(t: sea_orm_active_enums::TagType) -> Self {
        match t {
            sea_orm_active_enums::TagType::Genre => TagType::Genre,
            sea_orm_active_enums::TagType::Artist => TagType::Artist,
            sea_orm_active_enums::TagType::Origin => TagType::Origin,
            sea_orm_active_enums::TagType::Serial => TagType::Serial,
            sea_orm_active_enums::TagType::Chara => TagType::Chara,
            sea_orm_active_enums::TagType::Lang => TagType::Lang,
            sea_orm_active_enums::TagType::Group => TagType::Group, 
        }
    }
}

impl From<TagType> for sea_orm_active_enums::TagType {
    fn from(tag_type: TagType) -> Self {
        match tag_type {
            TagType::Genre => sea_orm_active_enums::TagType::Genre,
            TagType::Artist => sea_orm_active_enums::TagType::Artist,
            TagType::Origin => sea_orm_active_enums::TagType::Origin,
            TagType::Serial => sea_orm_active_enums::TagType::Serial,
            TagType::Chara => sea_orm_active_enums::TagType::Chara,
            TagType::Lang => sea_orm_active_enums::TagType::Lang,
            TagType::Group => sea_orm_active_enums::TagType::Group,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tasks {
    pub id: Option<i32>,
    pub status: Option<TaskStatus>,
    pub task: Option<Json>,
    pub ending_reason: Option<Option<String>>,
    pub create_time: Option<DateTimeWithTimeZone>,
    pub update_time: Option<DateTimeWithTimeZone>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tokens {
    pub id: Option<Uuid>,
    pub hash: Option<String>,
    pub remark: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub create_time: Option<DateTimeWithTimeZone>,
    pub revoke_time: Option<Option<DateTimeWithTimeZone>>,
    pub expire_time: Option<Option<DateTimeWithTimeZone>>,
    pub is_revoked: Option<bool>,
}

set!(Tokens => tokens::ActiveModel {
    id,
    hash,
    remark,
    description,
    create_time,
    revoke_time,
    expire_time,
    is_revoked,
});

set!(Metadata => metadata::ActiveModel {
    id,
    page_count,
    upload,
});

set!(Literatures => literatures::ActiveModel {
    id,
    metadata_id,
    title,
    description,
    lang,
});

set!(TagMetadata => tag_metadata::ActiveModel {
    metadata_id,
    tag_id,
});

set!(Tags => tags::ActiveModel {
    id,
    r#type: enum sea_orm_active_enums::TagType,
    label,
    canonical_id,
    ref_count,
});

// Tasks
set!(Tasks => tasks::ActiveModel {
    id,
    status: enum sea_orm_active_enums::TaskStatus,
    task,
    ending_reason,
    create_time,
    update_time,
});

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn serialize() {
        let a = json!({
            "id": 123,
            "status": "processing",
        });

        let b = serde_json::from_value::<Tasks>(a).unwrap();
        println!("{:?}", b.into_active_model());
    }
}
