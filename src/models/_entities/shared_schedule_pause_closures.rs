use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "shared_schedule_pause_closures")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub pause_instance_id: String,
    pub person_schedule_id: String,
    pub clinical_revision: i64,
    pub ended_at: DateTime,
    pub actor_account_id: i64,
    pub actor_person_id: i64,
    pub actor_membership_id: Option<i64>,
    pub guardian_declaration_id: Option<i64>,
    pub request_id: String,
    pub authentication_method: Option<String>,
    pub session_reference: Option<String>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
