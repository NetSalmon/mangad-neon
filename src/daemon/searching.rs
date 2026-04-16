use crate::daemon::models::errors::DaemonError;
use crate::daemon::models::searching::Document;
use mangad_neon::config::Config;
use mangad_neon::db::repository::Repository;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use sea_orm::sqlx::postgres::PgListener;
use std::sync::Arc;

pub async fn index(config: Arc<Config>) -> Result<Index, DaemonError> {
    let client = Client::new(&config.search.host, config.search.api_key.clone())?;
    let index = client.index("mangas");
    index
        .set_filterable_attributes([
            "genres",
            "artists",
            "groups",
            "languages",
            "characters",
            "serials",
            "origins",
        ])
        .await?;
    index
        .set_searchable_attributes(["description", "title"])
        .await?;

    Ok(index)
}

pub async fn sync(repo: Arc<Repository>, index: Arc<Index>) -> Result<(), DaemonError> {
    let inner_pool = repo.db.get_postgres_connection_pool();
    let mut listener = PgListener::connect_with(inner_pool).await?;
    listener
        .listen_all(vec!["literatures", "tags", "tag_metadata"])
        .await?;

    while let Ok(r) = listener.recv().await {
        let channel = r.channel();
        let payload = r.payload();
        let res: Result<(), DaemonError> = async {
            match channel {
                "literatures" | "tag_metadata" => {
                    let id: i32 = payload.parse().map_err(|_| {
                        DaemonError::CustomError("string parse to int error".to_string())
                    })?;
                    let (literatures, tags) = repo.select_literatures_and_tags(id).await?;
                    let docs: Vec<Document> = literatures
                        .into_iter()
                        .map(|literature| (literature, tags.clone()).into())
                        .collect();

                    index.add_documents(&docs, Some("id")).await?;
                }
                "tags" => {
                    let id: i32 = payload.parse().map_err(|_| {
                        DaemonError::CustomError("string parse to int error".to_string())
                    })?;

                    let ids = repo.select_metadata_id_by_tag_id(id).await?;
                    let mut docs: Vec<Document> = vec![];
                    for id in ids {
                        let (literature, tags) = repo.select_literatures_and_tags(id).await?;
                        literature
                            .into_iter()
                            .for_each(|literature| docs.push((literature, tags.clone()).into()));
                    }

                    index.add_documents(&docs, Some("id")).await?;
                }
                _ => {
                    return Err(DaemonError::CustomError("channel not support".to_string()));
                }
            }
            Ok(())
        }
        .await;

        if let Err(e) = res {
            return Err(e); // 以后再做进一步处理
        }
    }
    Ok(())
}
