INSERT INTO public.account_webauthn_user_ids(account_id,webauthn_id,created_at,updated_at)
SELECT 71001,'c3ludGhldGljLXVuc3VwcG9ydGVkLWhhbmRsZQ',now(),now()
WHERE NOT EXISTS(SELECT 1 FROM public.account_webauthn_user_ids WHERE account_id=71001);
INSERT INTO public.account_webauthn_keys(account_id,nickname,webauthn_id,public_key,sign_count,created_at,updated_at)
VALUES(71001,'Older unsupported passkey','b2xkZXItdW5zdXBwb3J0ZWQ',:'public_key',0,now(),now());
