INSERT INTO public.security_audit_events (
  id, household_id, actor_account_id, actor_membership_id, event_type,
  request_id, metadata, audit_context, created_at, updated_at
)
SELECT event_id, 72001, 71001, 74001, 'api.request', NULL,
  '{"http_method":"GET","controller":"api/v1/admin/audit_logs","action":"index","outcome":"success","status":200}'::jsonb,
  '{}'::jsonb, '2030-01-15 12:00:00', '2030-01-15 12:00:00'
FROM generate_series(957000, 957050) AS event_id;
