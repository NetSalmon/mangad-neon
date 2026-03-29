use crate::core::entities::dao::crawler::Task;
use crate::core::entities::orm::prelude::Tasks;
use crate::core::entities::orm::sea_orm_active_enums::TaskStatus;
use crate::core::entities::orm::tasks;
use crate::error::Error;
use sea_orm::ActiveModelTrait;
use sea_orm::{DatabaseConnection, EntityTrait, PrimaryKeyTrait, Set};

pub async fn find_by_id<E>(db: &DatabaseConnection, id: i32) -> Result<Option<E::Model>, Error>
where
    E: EntityTrait,
    <E::PrimaryKey as PrimaryKeyTrait>::ValueType: From<i32>,
{
    Ok(E::find_by_id(id).one(db).await?)
}

pub async fn insert_task(db: &DatabaseConnection, task: Task) -> Result<tasks::Model, Error> {
    let new = tasks::ActiveModel {
        task: Set(serde_json::to_value(&task)?),
        ..Default::default()
    };

    let resp = Tasks::insert(new).exec_with_returning(db).await?;

    Ok(resp)
}

pub async fn update_task_status(
    db: &DatabaseConnection,
    id: i32,
    task_status: TaskStatus,
) -> Result<tasks::Model, Error> {
    let new = tasks::ActiveModel {
        id: Set(id),
        status: Set(Some(task_status)),
        ..Default::default()
    };

    let resp = new.update(db).await?;

    Ok(resp)
}