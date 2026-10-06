UPDATE public.account_verification_keys
SET email_last_sent = clock_timestamp() - interval '6 minutes'
WHERE account_id IN (SELECT id FROM public.accounts WHERE email = :'email');
