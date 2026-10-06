INSERT INTO public.accounts(id, email, password_hash, created_at, updated_at)
VALUES (71001, 'persistence@example.test', crypt('password', gen_salt('bf', 4)), now(), now());

INSERT INTO public.households(id, created_by_account_id, name, slug, timezone, created_at, updated_at)
VALUES (72001, 71001, 'Synthetic household', 'persistence-fixture', 'Europe/London', now(), now());

INSERT INTO public.people(id, account_id, household_id, name, person_type, has_capacity, created_at, updated_at)
VALUES (73001, 71001, 72001, 'Synthetic adult', 0, true, now(), now()),
       (73002, NULL, 72001, 'Synthetic minor', 1, false, now(), now()),
       (73003, NULL, 72001, 'Synthetic dependent adult', 2, false, now(), now());

INSERT INTO public.household_memberships(id, account_id, household_id, person_id, role, joined_at, created_at, updated_at)
VALUES (74001, 71001, 72001, 73001, 'administrator', now(), now(), now());

INSERT INTO public.oauth_applications(id, account_id, client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, created_at, updated_at)
VALUES (75001, 71001, 'synthetic-client', 'integration', 'Synthetic client', 'https://example.test/callback', 'patient/*.read', 'none', now(), now());

INSERT INTO public.oauth_grants(id, account_id, oauth_application_id, household_membership_id, person_id, permissions_version, code, token, token_hash, refresh_token, refresh_token_hash, scopes, expires_in, created_at, updated_at)
VALUES (76001, 71001, 75001, 74001, 73001, 1, 'synthetic-code', 'synthetic-token', 'synthetic-token-hash', 'synthetic-refresh', 'synthetic-refresh-hash', 'patient/*.read', now() + interval '1 hour', now(), now());
