use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token_type: String,
    pub scope: String,
    pub expires_in: i64,
    pub ext_expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphListResponse {
    pub(crate) value: Vec<GraphDriveItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphDriveItem {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) size: u64,
    #[serde(rename = "lastModifiedDateTime")]
    pub(crate) last_modified: Option<String>,
    pub(crate) folder: Option<GraphFolderFacet>,
    pub(crate) file: Option<GraphFileFacet>,
    #[serde(rename = "@microsoft.graph.downloadUrl")]
    pub(crate) download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphFolderFacet {
    #[serde(rename = "childCount")]
    pub(crate) child_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphFileFacet {
    #[serde(rename = "mimeType")]
    pub(crate) mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub item_type: FileListItemType,
    pub last_modified: Option<String>,
    pub child_count: Option<u64>,
    pub mime_type: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileListItemType {
    File,
    Folder,
}

impl From<GraphDriveItem> for FileListItem {
    fn from(item: GraphDriveItem) -> Self {
        let child_count = item.folder.as_ref().and_then(|folder| folder.child_count);
        let mime_type = item.file.as_ref().and_then(|file| file.mime_type.clone());
        let item_type = if item.folder.is_some() {
            FileListItemType::Folder
        } else {
            FileListItemType::File
        };

        Self {
            id: item.id,
            name: item.name,
            size: item.size,
            item_type,
            last_modified: item.last_modified,
            child_count,
            mime_type,
            download_url: item.download_url,
        }
    }
}
