use crate::error::Error;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;

pub async fn search() -> Result<Index, Error> {
    let client = Client::new("http://127.0.0.1:7700", None::<String>)?;
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
