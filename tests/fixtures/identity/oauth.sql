UPDATE public.accounts SET status = 2;
INSERT INTO public.users(id, person_id, email_address, password_digest, created_at, updated_at)
VALUES (77001, 73001, 'persistence@example.test', crypt('password', gen_salt('bf', 4)), now(), now());
UPDATE public.oauth_applications SET client_id = 'native', redirect_uri = 'https://example.test/oauth/callback', scopes = 'patient/*.rs offline_access' WHERE id = 75001;
UPDATE public.oauth_grants SET code = 'synthetic-legacy-code', code_challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method = 'S256', redirect_uri = 'https://example.test/oauth/callback', scopes = 'patient/*.rs offline_access', token = NULL, refresh_token = NULL, token_hash = NULL, refresh_token_hash = NULL, expires_in = timezone('UTC', clock_timestamp()) + interval '5 minutes' WHERE id = 76001;
