use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "onedrive_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub onedrive_root_path: String,
    pub onedrive_client_id: String,
    pub onedrive_client_secret: String,
    pub onedrive_refresh_token: String,
}

impl ActiveModelBehavior for ActiveModel {}
