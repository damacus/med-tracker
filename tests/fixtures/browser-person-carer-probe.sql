SELECT json_build_object(
  'person_type',(SELECT person_type FROM public.people WHERE id=73002),
  'has_capacity',(SELECT has_capacity FROM public.people WHERE id=73002),
  'active_carers',(SELECT count(*) FROM public.carer_relationships WHERE patient_id=73002 AND active),
  'delegated_access',EXISTS(SELECT 1 FROM public.person_access_grants WHERE household_membership_id=74002 AND person_id=73002 AND revoked_at IS NULL),
  'independent_manage',EXISTS(SELECT 1 FROM public.person_access_grants WHERE id=78005 AND access_level='manage' AND revoked_at IS NULL AND carer_relationship_id IS NULL),
  'relationship_audits',(SELECT count(*) FROM public.versions WHERE item_type='CarerRelationship')
);
