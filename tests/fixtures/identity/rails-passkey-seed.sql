INSERT INTO public.account_webauthn_user_ids(account_id,webauthn_id,created_at,updated_at)
VALUES(71001,:'user_handle',now(),now());
INSERT INTO public.account_webauthn_keys(account_id,nickname,webauthn_id,public_key,sign_count,created_at,updated_at)
VALUES(71001,'Retained Rails device',:'webauthn_id',:'public_key',:'sign_count'::integer,now(),now());
