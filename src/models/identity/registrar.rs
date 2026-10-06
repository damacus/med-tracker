use async_trait::async_trait;
use oxide_auth::primitives::{
    registrar::{
        BoundClient, Client, ClientMap, ClientUrl, PreGrant, RegisteredUrl, Registrar,
        RegistrarError,
    },
    scope::Scope,
};

use super::{
    ExchangeError,
    input::{Input, Method},
};
use crate::models::entities::oauth_application;

pub(super) struct RegisteredClient {
    inner: ClientMap,
    pub application: oauth_application::Model,
    method: Method,
    submitted_id: String,
}

impl RegisteredClient {
    pub fn new(
        application: oauth_application::Model,
        input: &Input,
    ) -> Result<Self, ExchangeError> {
        let redirect = RegisteredUrl::Exact(
            application
                .redirect_uri
                .parse()
                .map_err(|_| ExchangeError::Unavailable)?,
        );
        let scope = application
            .scopes
            .parse()
            .map_err(|_| ExchangeError::Unavailable)?;
        let mut inner = ClientMap::new();
        inner.register_client(Client::public(&application.client_id, redirect, scope));
        Ok(Self {
            inner,
            application,
            method: input.method,
            submitted_id: input.id.clone(),
        })
    }
}

#[async_trait]
impl oxide_auth_async::primitives::Registrar for RegisteredClient {
    async fn bound_redirect<'a>(
        &self,
        bound: ClientUrl<'a>,
    ) -> Result<BoundClient<'a>, RegistrarError> {
        Registrar::bound_redirect(&self.inner, bound)
    }

    async fn negotiate<'a>(
        &self,
        client: BoundClient<'a>,
        scope: Option<Scope>,
    ) -> Result<PreGrant, RegistrarError> {
        Registrar::negotiate(&self.inner, client, scope)
    }

    async fn check(&self, id: &str, secret: Option<&[u8]>) -> Result<(), RegistrarError> {
        if id != self.submitted_id
            || id != self.application.client_id
            || !self
                .application
                .token_endpoint_auth_method
                .split_whitespace()
                .any(|method| method == self.method.registered_name())
        {
            return Err(RegistrarError::Unspecified);
        }
        if self.method == Method::None {
            return if secret.is_none() {
                Ok(())
            } else {
                Err(RegistrarError::Unspecified)
            };
        }
        let secret = secret.ok_or(RegistrarError::Unspecified)?;
        let valid = if let Some(hash) = self
            .application
            .client_secret_hash
            .as_ref()
            .filter(|hash| !hash.is_empty())
        {
            let hash = hash.clone();
            let secret = secret.to_vec();
            tokio::task::spawn_blocking(move || bcrypt::verify(secret, &hash))
                .await
                .map_err(|_| RegistrarError::PrimitiveError)?
                .map_err(|_| RegistrarError::PrimitiveError)?
        } else {
            false
        };
        if valid {
            Ok(())
        } else {
            Err(RegistrarError::Unspecified)
        }
    }
}
