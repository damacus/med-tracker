use better_auth::{
    AuthSchema,
    prelude::{AccountView, SessionView, UserView, VerificationView},
};

pub struct ClinicalAuthSchema;

impl AuthSchema for ClinicalAuthSchema {
    type User = UserView;
    type Session = SessionView;
    type Account = AccountView;
    type Verification = VerificationView;
}
