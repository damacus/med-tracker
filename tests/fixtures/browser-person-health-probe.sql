SELECT json_build_object(
  'events',(SELECT count(*) FROM public.health_events),
  'links',(SELECT count(*) FROM public.health_event_medications),
  'audits',(SELECT count(*) FROM public.versions WHERE item_type='HealthEvent'),
  'changes',(SELECT count(*) FROM public.api_change_events WHERE record_type='HealthEvent'),
  'tombstones',(SELECT count(*) FROM public.api_tombstones WHERE record_type='HealthEvent'),
  'rich_audits',(SELECT count(*) FROM public.versions WHERE item_type='HealthEvent' AND object_changes::jsonb->'action_taken'->>1='Synthetic rest' AND object_changes::jsonb->'medical_help_sought'->>1='true')
);
