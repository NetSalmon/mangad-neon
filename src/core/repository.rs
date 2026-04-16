use crate::core::config::Config;
use crate::core::dao::ExpireTime;
use crate::core::dao::{FullData, InlineLiterature, InlineTag, Literature, Tag, Task};
use crate::core::orm::prelude::Tasks;
use crate::core::orm::sea_orm_active_enums::{TagType, TaskStatus};
use crate::core::orm::{literatures, metadata, tag_metadata, tags, tasks, tokens};
use crate::core::token;
use crate::core::token::TokenTrait;
use crate::error::Error;
use chrono::Utc;
use paste::paste;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, PaginatorTrait, QueryFilter,
    TransactionTrait, sea_query,
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;

pub struct Repository {
    pub db: DatabaseConnection,
}

pub trait IntoDatabaseUrl {
    fn to_database_url(&self) -> String;
}

impl IntoDatabaseUrl for Arc<Config> {
    fn to_database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.database,
        )
    }
}

impl IntoDatabaseUrl for Config {
    fn to_database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.database,
        )
    }
}

impl Repository {
    pub async fn new(url: &str) -> Result<Self, Error> {
        tracing::debug!("Connecting to database...");
        let db = sea_orm::Database::connect(url).await?;
        tracing::info!("Database connected successfully");

        Ok(Self { db })
    }

    pub async fn insert_task(&self, task: &Task) -> Result<tasks::Model, Error> {
        tracing::debug!("Inserting new task into database");
        let new = tasks::ActiveModel {
            task: Set(serde_json::to_value(&task)?),
            ..Default::default()
        };

        let resp = Tasks::insert(new).exec_with_returning(&self.db).await?;
        tracing::debug!("Task inserted with ID: {}", resp.id);

        Ok(resp)
    }

    pub async fn update_task_status(
        &self,
        id: i32,
        task_status: TaskStatus,
    ) -> Result<tasks::Model, Error> {
        tracing::debug!("Updating task {} status to {:?}", id, task_status);
        let new = tasks::ActiveModel {
            id: Set(id),
            status: Set(task_status),
            ..Default::default()
        };

        let resp = new.update(&self.db).await?;

        Ok(resp)
    }

    pub async fn update_task_status_with_reason(
        &self,
        id: i32,
        task_status: TaskStatus,
        reason: &str,
    ) -> Result<tasks::Model, Error> {
        tracing::warn!(
            "Task {} failed: {:?}. Updating status to {:?}",
            id,
            reason,
            task_status
        );
        let new = tasks::ActiveModel {
            id: Set(id),
            status: Set(task_status),
            ending_reason: Set(Some(reason.to_string())),
            ..Default::default()
        };

        let resp = new.update(&self.db).await?;

        Ok(resp)
    }
}

impl Repository {
    pub async fn insert_manga_from_task(&self, task: &Task) -> Result<metadata::Model, Error> {
        let tx = self.db.begin().await?;
        let tags = Self::select_or_insert_tags(&tx, &task.tags).await?;

        let metadata = metadata::ActiveModel {
            page_count: Set(task.images.len() as i32),
            upload: Default::default(),
            ..Default::default()
        };

        let resp = metadata::Entity::insert(metadata)
            .exec_with_returning(&tx)
            .await?;
        let id = resp.id;

        Self::insert_tag_metadata(&tx, id, &tags).await?;

        Self::insert_literatures(&tx, id, &task.literatures).await?;

        tx.commit().await?;

        Ok(resp)
    }

    pub async fn insert_tag_metadata(
        tx: &DatabaseTransaction,
        mid: i32,
        tags: &Vec<tags::Model>,
    ) -> Result<Vec<tag_metadata::Model>, Error> {
        if tags.is_empty() {
            return Ok(vec![]);
        }
        let active_models: Vec<tag_metadata::ActiveModel> = tags
            .iter()
            .map(|tag| tag_metadata::ActiveModel {
                metadata_id: Set(mid),
                tag_id: Set(tag.id),
                weight: Default::default(),
            })
            .collect();
        let resp = tag_metadata::Entity::insert_many(active_models)
            .exec_with_returning_many(tx)
            .await?;
        Ok(resp)
    }

    pub async fn insert_literatures(
        tx: &DatabaseTransaction,
        mid: i32,
        literatures: &Vec<Literature>,
    ) -> Result<Vec<literatures::Model>, Error> {
        if literatures.is_empty() {
            return Ok(vec![]);
        }
        let active_models: Vec<literatures::ActiveModel> = literatures
            .into_iter()
            .map(|literature| literatures::ActiveModel {
                metadata_id: Set(mid),
                title: Set(literature.title.clone()),
                description: Set(literature.description.clone()),
                lang: {
                    if let Some(ref lang) = literature.lang {
                        Set(lang.clone())
                    } else {
                        Default::default()
                    }
                },
                ..Default::default()
            })
            .collect();

        let result = literatures::Entity::insert_many(active_models)
            .on_conflict(
                sea_query::OnConflict::columns([
                    literatures::Column::Title,
                    literatures::Column::MetadataId,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec_with_returning_many(tx)
            .await?;

        Ok(result)
    }

    pub async fn select_or_insert_tags(
        tx: &DatabaseTransaction,
        tags: &Vec<Tag>,
    ) -> Result<Vec<tags::Model>, Error> {
        if tags.is_empty() {
            return Ok(vec![]);
        }
        let active_models: Vec<tags::ActiveModel> = tags
            .into_iter()
            .map(|tag| tags::ActiveModel {
                r#type: Set(tag.r#type.clone().into()),
                label: Set(tag.label.clone()),
                canonical_id: Set(tag.canonical_id.to_owned()),
                ..Default::default()
            })
            .collect();

        tags::Entity::insert_many(active_models)
            .on_conflict(
                sea_query::OnConflict::columns([tags::Column::Type, tags::Column::Label])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_without_returning(tx)
            .await?;

        let mut condition = Condition::any();
        for item in tags {
            condition = condition.add(
                Condition::all()
                    .add(tags::Column::Type.eq::<TagType>(item.r#type.clone().into()))
                    .add(tags::Column::Label.eq(item.label.clone())),
            );
        }

        let result = tags::Entity::find().filter(condition).all(tx).await?;

        Ok(result)
    }

    pub async fn verify_token(&self, token: &str) -> Result<bool, Error> {
        let uuid = token::get_uuid(&token)?;
        let Some(model) = tokens::Entity::find_by_id(uuid).one(&self.db).await? else {
            return Ok(false);
        };

        if model.is_revoked {
            return Ok(false);
        }

        let is_expired = match model.expire_time {
            None => false,
            Some(t) => Utc::now() > t,
        };

        if is_expired {
            return Ok(false);
        }

        Ok(token::verify_hash(&token, &model.hash)?)
    }

    pub async fn create_token(
        &self,
        expire_time: ExpireTime,
        remark: Option<String>,
        description: Option<String>,
    ) -> Result<(tokens::Model, String), Error> {
        let (token, uuid) = token::gen_token();
        let now: DateTimeWithTimeZone = Utc::now().with_timezone(&Utc).into();
        let expire = expire_time.get_expire_time(now);
        let active = tokens::ActiveModel {
            id: Set(uuid),
            hash: Set(token::hash(&token)?),
            remark: Set(remark),
            description: Set(description),
            create_time: Set(now).into(),
            expire_time: Set(expire).into(),
            ..Default::default()
        };

        let model = tokens::Entity::insert(active)
            .exec_with_returning(&self.db)
            .await?;

        Ok((model, token))
    }

    pub async fn revoke_token<T>(&self, t: &T) -> Result<(), Error>
    where
        T: TokenTrait,
    {
        let uuid = t.uuid()?;
        let active = tokens::ActiveModel {
            id: Set(uuid),
            is_revoked: Set(true),
            revoke_time: Set(Some(Utc::now().into())),
            ..Default::default()
        };

        tokens::Entity::update(active).exec(&self.db).await?;

        Ok(())
    }

    pub async fn select_literatures_and_tags(
        &self,
        id: i32,
    ) -> Result<(Vec<literatures::Model>, Vec<tags::Model>), Error> {
        let tx = self.db.begin().await?;
        let tag_id = tag_metadata::Entity::find()
            .filter(tag_metadata::Column::MetadataId.eq(id))
            .all(&tx)
            .await?
            .into_iter()
            .map(|m| m.tag_id)
            .collect::<Vec<i32>>();

        let tags = tags::Entity::find()
            .filter(tags::Column::Id.is_in(tag_id))
            .all(&tx)
            .await?;

        let literatures = literatures::Entity::find()
            .filter(literatures::Column::MetadataId.eq(id))
            .all(&tx)
            .await?;

        Ok((literatures, tags))
    }

    pub async fn select_metadata_id_by_tag_id(&self, id: i32) -> Result<Vec<i32>, Error> {
        let id = tag_metadata::Entity::find()
            .filter(tag_metadata::Column::TagId.eq(id))
            .all(&self.db)
            .await?
            .iter()
            .map(|m| m.metadata_id)
            .collect::<Vec<i32>>();

        Ok(id)
    }
}
macro_rules! list {
    ($path:path, $name:ident) => {
        paste! {
            pub async fn [<list_ $name>] (
                &self,
                size: u64,
                offset: u64,
            ) -> Result<Vec<$path::Model>, Error> {
                let models = $path::Entity::find()
                    .paginate(&self.db, size)
                    .fetch_page(offset)
                    .await?;

                Ok(models)
            }
        }
    };
}

impl Repository {
    list!(tags, tags);
    list!(literatures, literatures);
    list!(tag_metadata, tag_metadata);
    list!(tokens, tokens);
    list!(metadata, metadata);
    list!(tasks, tasks);
    pub async fn list_full_data(&self, size: u64, offset: u64) -> Result<Vec<FullData>, Error> {
        let tx = self.db.begin().await?;
        let ids = metadata::Entity::find()
            .paginate(&tx, size)
            .fetch_page(offset)
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<i32>>();

        let mut data: Vec<FullData> = Vec::with_capacity(ids.len());

        for (i, id) in ids.into_iter().enumerate() {
            data.insert(i, select_full_data_with_tx(id, &tx).await?);
        }

        tx.commit().await?;

        Ok(data)
    }
}

pub async fn select_full_data_with_tx(
    id: i32,
    tx: &DatabaseTransaction,
) -> Result<FullData, Error> {
    tracing::debug!("select full data with {}", id);

    let m = metadata::Entity::find_by_id(id)
        .one(tx)
        .await?
        .ok_or(Error::NotFound)?;

    let (mt, weights): (Vec<i32>, Vec<i32>) = tag_metadata::Entity::find()
        .filter(tag_metadata::Column::MetadataId.eq(id))
        .all(tx)
        .await?
        .into_iter()
        .map(|tag| (tag.tag_id, tag.weight))
        .unzip();

    let tags = tags::Entity::find()
        .filter(tags::Column::Id.is_in(mt))
        .all(tx)
        .await?
        .into_iter()
        .zip(weights)
        .map(|t| t.into())
        .collect::<Vec<InlineTag>>();

    let literatures = literatures::Entity::find()
        .filter(literatures::Column::MetadataId.eq(id))
        .all(tx)
        .await?
        .into_iter()
        .map(|t| t.into())
        .collect::<Vec<InlineLiterature>>();

    let fin = FullData {
        id: m.id,
        rating: m.rating,
        page_count: m.page_count,
        upload: m.upload,
        literatures,
        tags,
    };

    Ok(fin)
}
