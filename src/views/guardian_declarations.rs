use crate::models::care::guardian_declarations::OwnDeclaration;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct DeclarationPage {
    pub(crate) data: Vec<OwnDeclaration>,
    pub(crate) meta: DeclarationCursor,
}

#[derive(Serialize)]
pub(crate) struct DeclarationCursor {
    pub(crate) next_before_id: Option<String>,
}
