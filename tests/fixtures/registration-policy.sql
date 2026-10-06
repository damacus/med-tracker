DELETE FROM public.app_settings;
INSERT INTO public.app_settings(invite_only,created_at,updated_at) VALUES(:'invite_only'::boolean,now(),now());
