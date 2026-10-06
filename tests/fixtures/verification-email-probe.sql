SELECT json_build_object(
  'keys', (SELECT count(*) FROM public.account_verification_keys WHERE account_id IN (SELECT id FROM public.accounts WHERE email = :'email')),
  'mail_jobs', (SELECT count(*) FROM public.pg_loco_queue WHERE name = 'MailerWorker' AND task_data->>'to' = :'email')
);
