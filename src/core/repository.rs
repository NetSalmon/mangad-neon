use crate::core::entities::config::Config;
use crate::core::entities::dao::crawler::{Literature, Tag, Task};
use crate::core::entities::inner::ExpireTime;
use crate::core::entities::orm::prelude::Tasks;
use crate::core::entities::orm::sea_orm_active_enums::TaskStatus;
use crate::core::entities::orm::{literatures, metadata, tag_metadata, tags, tasks, tokens};
use crate::core::token;
use crate::core::token::TokenTrait;
use crate::error::Error;
use chrono::Utc;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, QueryFilter, TransactionTrait,
    sea_query,
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;

pub struct Repository {
    db: DatabaseConnection,
}

impl Repository {
    pub async fn new(config: Arc<Config>) -> Result<Self, Error> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.database.user,
            config.database.password,
            config.database.host,
            config.database.port,
            config.database.database,
        );

        println!("Connecting to {}", url);

        let db = sea_orm::Database::connect(&url).await?;

        Ok(Self { db })
    }

    pub async fn insert_task(&self, task: &Task) -> Result<tasks::Model, Error> {
        let new = tasks::ActiveModel {
            task: Set(serde_json::to_value(&task)?),
            ..Default::default()
        };

        let resp = Tasks::insert(new).exec_with_returning(&self.db).await?;

        Ok(resp)
    }

    pub async fn update_task_status(
        &self,
        id: i32,
        task_status: TaskStatus,
    ) -> Result<tasks::Model, Error> {
        let new = tasks::ActiveModel {
            id: Set(id),
            status: Set(Some(task_status)),
            ..Default::default()
        };

        let resp = new.update(&self.db).await?;

        Ok(resp)
    }

    pub async fn update_task_status_with_reason(
        &self,
        id: i32,
        task_status: TaskStatus,
        reason: Error,
    ) -> Result<tasks::Model, Error> {
        let new = tasks::ActiveModel {
            id: Set(id),
            status: Set(Some(task_status)),
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
                r#type: Set(tag.r#type.to_orm()),
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
                    .add(tags::Column::Type.eq(item.r#type.to_orm()))
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
        let Some(expire_time) = model.expire_time else {
            return Ok(true);
        };
        if Utc::now() > expire_time.with_timezone(&Utc) {
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
}

#[cfg(test)]
mod tests {
    use crate::core::repository::*;
    #[tokio::test]
    async fn t() {
        let config = Arc::new(
            toml::from_str(&std::fs::read_to_string("./config/config.toml").unwrap()).unwrap(),
        );

        let repo = Arc::new(Repository::new(config).await.unwrap());

        let (_, token) = repo
            .create_token(ExpireTime::Never, Some("test".to_string()), None)
            .await
            .unwrap();

        println!("{:#?}", token);

        println!("{:#?}", repo.verify_token(&token).await.unwrap());
    }
}
