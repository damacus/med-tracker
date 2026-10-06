INSERT INTO public.oauth_applications(id, account_id, client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, created_at, updated_at)
VALUES (75002, 71001, 'native-journey', 'mobile', 'Synthetic native journey', 'io.damacus.medtracker:/oauth2redirect', 'medtracker offline_access', 'none', now(), now());

INSERT INTO public.oauth_applications(id, account_id, client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, client_secret_hash, created_at, updated_at)
VALUES (75003, 71001, 'smart-journey', 'integration', 'Synthetic SMART journey', 'https://example.test/callback?tenant=7', 'launch/patient patient/*.rs offline_access', 'client_secret_post', crypt('password', gen_salt('bf', 4)), now(), now());
