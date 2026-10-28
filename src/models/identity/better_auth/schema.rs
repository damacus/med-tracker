use better_auth_core::AuthSchema;

pub(crate) mod account;
pub mod api_key;
pub mod passkey;
pub(crate) mod rate_limit;
pub mod session;
pub(crate) mod two_factor;
pub(crate) mod user;
pub mod verification;

pub struct AccountSchema;

impl AuthSchema for AccountSchema {
    type User = user::Model;
    type Session = session::Model;
    type Account = account::Model;
    type Verification = verification::Model;
}

pub(crate) type PluginSchema = better_auth_seaorm::PluginModels<
    api_key::Model,
    better_auth_seaorm::store::entities::device_code::Model,
    passkey::Model,
    two_factor::Model,
    better_auth_seaorm::store::entities::jwk::Model,
    better_auth_seaorm::store::entities::wallet_address::Model,
    rate_limit::Model,
>;
