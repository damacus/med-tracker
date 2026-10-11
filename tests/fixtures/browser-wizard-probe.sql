SELECT json_build_object(
  'id', m.id,
  'dmd_system', m.dmd_system,
  'default_schedule_type', m.default_schedule_type,
  'default_schedule_config', m.default_schedule_config,
  'administration_kind', (SELECT pm.administration_kind FROM public.person_medications pm WHERE pm.medication_id=m.id ORDER BY pm.id DESC LIMIT 1)
)
FROM public.medications m
WHERE m.name=:'name'
ORDER BY m.id DESC
LIMIT 1;
