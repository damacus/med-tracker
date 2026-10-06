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
    pub fn authorization(application: oauth_application::Model) -> Result<Self, ExchangeError> {
        let id = application.client_id.clone();
        Self::build(application, Method::None, id)
    }

    pub fn new(
        application: oauth_application::Model,
        input: &Input,
    ) -> Result<Self, ExchangeError> {
        Self::build(application, input.method, input.id.clone())
    }

    fn build(
        application: oauth_application::Model,
        method: Method,
        submitted_id: String,
    ) -> Result<Self, ExchangeError> {
        let mut redirects = application.redirect_uri.split_whitespace().map(|value| {
            value
                .parse()
                .map(RegisteredUrl::Exact)
                .map_err(|_| ExchangeError::Unavailable)
        });
        let redirect = redirects.next().ok_or(ExchangeError::Unavailable)??;
        let additional = redirects.collect::<Result<Vec<_>, _>>()?;
        let scope = application
            .scopes
            .parse()
            .map_err(|_| ExchangeError::Unavailable)?;
        let mut inner = ClientMap::new();
        inner.register_client(
            Client::public(&application.client_id, redirect, scope)
                .with_additional_redirect_uris(additional),
        );
        Ok(Self {
            inner,
            application,
            method,
            submitted_id,
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
        let mut granted = Registrar::negotiate(&self.inner, client, None)?;
        if let Some(requested) = scope {
            if !granted.scope.priviledged_to(&requested) {
                return Err(RegistrarError::Unspecified);
            }
            granted.scope = requested;
        }
        Ok(granted)
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
