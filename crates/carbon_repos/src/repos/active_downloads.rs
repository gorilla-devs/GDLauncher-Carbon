//! Repository queries for the `ActiveDownloads` table.

use crate::queries;

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ActiveDownloadRow {
    pub url: String,
    #[column("file_id")]
    pub file_id: String,
}

queries! {
    fn find_active_download_by_url(url: &str) -> Option<ActiveDownloadRow> =
        "SELECT url, file_id FROM ActiveDownloads WHERE url = :url";
    fn delete_active_download_by_file_id(file_id: &str) -> usize =
        "DELETE FROM ActiveDownloads WHERE file_id = :file_id";
    fn insert_active_download(url: &str, file_id: &str) -> usize =
        "INSERT INTO ActiveDownloads (url, file_id) VALUES (:url, :file_id)";
}

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    QUERIES.to_vec()
}
