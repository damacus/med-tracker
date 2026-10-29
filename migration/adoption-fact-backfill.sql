-- Migration backfill of adoption facts for sources linked before the feature
-- existed. This is a trigger/RLS boundary block: the runtime validation
-- triggers cannot evaluate without tenant GUCs, and SeaQuery has no trigger
-- toggle, so both validation triggers are disabled only around static
-- INSERT...SELECT statements that re-apply the exact same evidence rules.
-- No consent, stock, outbox or operational replay side effects are produced;
-- every NOT EXISTS keeps the whole block idempotent.
ALTER TABLE public.shared_schedule_adoption_facts DISABLE TRIGGER adoption_fact_guard;
ALTER TABLE public.shared_schedule_administrations DISABLE TRIGGER shared_administration_guard;

INSERT INTO public.shared_schedule_adoption_facts (
 medication_take_id,person_schedule_id,source_schedule_id,source_portable_id,
 person_id,household_id,take_portable_id,original_version_id,client_uuid,
 taken_at,dose_amount,dose_unit,taken_from_medication_id,taken_from_location_id,
 take_created_at,take_updated_at,fact_hash,captured_by_account_id,
 captured_by_person_id,captured_by_membership_id,guardian_declaration_id,
 request_id,authentication_method,credential_reference,captured_at)
SELECT evidence.medication_take_id,evidence.person_schedule_id,evidence.source_schedule_id,evidence.source_portable_id,
 evidence.person_id,evidence.household_id,evidence.take_portable_id,evidence.original_version_id,evidence.client_uuid,
 evidence.taken_at,evidence.dose_amount,evidence.dose_unit,evidence.taken_from_medication_id,evidence.taken_from_location_id,
 evidence.take_created_at,evidence.take_updated_at,
 med_tracker.adoption_fact_hash(ROW(evidence.medication_take_id,evidence.source_schedule_id,evidence.person_id,
  evidence.household_id,evidence.original_version_id,evidence.captured_by_account_id,evidence.captured_by_person_id,
  evidence.captured_by_membership_id,evidence.person_schedule_id,evidence.source_portable_id,evidence.take_portable_id,
  evidence.request_id,evidence.client_uuid,evidence.dose_unit,evidence.authentication_method,evidence.credential_reference,
  evidence.taken_from_medication_id,evidence.taken_from_location_id,NULL::bigint,evidence.taken_at,evidence.take_created_at,
  evidence.take_updated_at,evidence.captured_at,evidence.dose_amount,'\\x'::bytea)::public.shared_schedule_adoption_facts),
 evidence.captured_by_account_id,evidence.captured_by_person_id,evidence.captured_by_membership_id,NULL,
 evidence.request_id,evidence.authentication_method,evidence.credential_reference,evidence.captured_at
FROM (
 SELECT take.id AS medication_take_id, source.person_schedule_id, source.id AS source_schedule_id,
  source.portable_id AS source_portable_id, source.person_id, source.household_id,
  take.portable_id AS take_portable_id, original.id AS original_version_id, take.client_uuid,
  take.taken_at, take.dose_amount, take.dose_unit, take.taken_from_medication_id, take.taken_from_location_id,
  take.created_at AS take_created_at, take.updated_at AS take_updated_at,
  (adoption.audit_context->>'actor_account_id')::bigint AS captured_by_account_id,
  (adoption.audit_context->>'actor_person_id')::bigint AS captured_by_person_id,
  (adoption.audit_context->>'actor_membership_id')::bigint AS captured_by_membership_id,
  adoption.request_id AS request_id,
  adoption.audit_context->>'authentication_method' AS authentication_method,
  substring(adoption.audit_context->>'session_reference' from '^(?:personal_api_key|api_session|api_app_token|oauth_grant|browser_session):(.*)$') AS credential_reference,
  adoption.created_at AS captured_at
 FROM public.medication_takes take
 JOIN public.schedules source ON source.id=take.schedule_id AND source.household_id=take.household_id
 JOIN public.person_schedules plan ON plan.id=source.person_schedule_id
  AND plan.origin_schedule_id=source.id AND plan.origin_household_id=source.household_id
  AND plan.person_id=source.person_id
 JOIN LATERAL (SELECT version.* FROM public.versions version
  WHERE version.item_type='MedicationTake' AND version.item_id=take.id AND version.event='create'
  ORDER BY version.id LIMIT 1) original ON true
 JOIN LATERAL (SELECT CASE WHEN original.object IS NOT NULL THEN original.object::jsonb
  ELSE (SELECT jsonb_object_agg(change.key,change.value->1)
   FROM jsonb_each(original.object_changes::jsonb) change
   WHERE jsonb_typeof(change.value)='array' AND jsonb_array_length(change.value)=2
    AND change.value->0='null'::jsonb) END AS payload) original_fact ON true
 JOIN LATERAL (SELECT version.* FROM public.versions version
  WHERE version.item_type='PersonSchedule' AND version.item_id=source.id AND version.event='create'
  ORDER BY version.id LIMIT 1) adoption ON true
 WHERE take.person_medication_id IS NULL AND take.taken_at IS NOT NULL AND take.dose_amount>0
  AND original.household_id IS NOT DISTINCT FROM take.household_id
  AND original.whodunnit IS NOT DISTINCT FROM original.audit_context->>'actor_account_id'
  AND original.actor_membership_id::text IS NOT DISTINCT FROM original.audit_context->>'actor_membership_id'
  AND original.household_id::text IS NOT DISTINCT FROM original.audit_context->>'household_id'
  AND original.request_id IS NOT DISTINCT FROM original.audit_context->>'request_id'
  AND (original_fact.payload->>'id')::bigint IS NOT DISTINCT FROM take.id
  AND original_fact.payload->>'portable_id' IS NOT DISTINCT FROM take.portable_id
  AND original_fact.payload->>'client_uuid' IS NOT DISTINCT FROM take.client_uuid
  AND (original_fact.payload->>'household_id')::bigint IS NOT DISTINCT FROM take.household_id
  AND (original_fact.payload->>'schedule_id')::bigint IS NOT DISTINCT FROM take.schedule_id
  AND (original_fact.payload->>'person_medication_id')::bigint IS NOT DISTINCT FROM take.person_medication_id
  AND (original_fact.payload->>'taken_from_medication_id')::bigint IS NOT DISTINCT FROM take.taken_from_medication_id
  AND (original_fact.payload->>'taken_from_location_id')::bigint IS NOT DISTINCT FROM take.taken_from_location_id
  AND (original_fact.payload->>'dose_amount')::numeric IS NOT DISTINCT FROM take.dose_amount
  AND original_fact.payload->>'dose_unit' IS NOT DISTINCT FROM take.dose_unit
  AND med_tracker.adoption_audit_time_matches(original_fact.payload->>'taken_at',take.taken_at)
  AND med_tracker.adoption_audit_time_matches(original_fact.payload->>'created_at',take.created_at)
  AND med_tracker.adoption_audit_time_matches(original_fact.payload->>'updated_at',take.updated_at)
  AND adoption.household_id IS NOT DISTINCT FROM take.household_id
  AND adoption.whodunnit IS NOT DISTINCT FROM adoption.audit_context->>'actor_account_id'
  AND adoption.actor_membership_id::text IS NOT DISTINCT FROM adoption.audit_context->>'actor_membership_id'
  AND adoption.household_id::text IS NOT DISTINCT FROM adoption.audit_context->>'household_id'
  AND adoption.request_id IS NOT DISTINCT FROM adoption.audit_context->>'request_id'
  AND adoption.audit_context->>'actor_account_id' IS NOT NULL
  AND adoption.audit_context->>'actor_person_id' IS NOT NULL
  AND adoption.audit_context->>'actor_membership_id' IS NOT NULL
  AND adoption.request_id IS NOT NULL
  AND NOT EXISTS(SELECT 1 FROM public.shared_schedule_adoption_facts existing WHERE existing.medication_take_id=take.id)
  AND NOT EXISTS(SELECT 1 FROM public.shared_schedule_administrations existing WHERE existing.medication_take_id=take.id)
  AND EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=(adoption.audit_context->>'actor_account_id')::bigint
   AND actor.person_id=(adoption.audit_context->>'actor_person_id')::bigint)
) evidence;

INSERT INTO public.shared_schedule_administrations (person_schedule_id,household_id,medication_take_id,taken_at)
SELECT fact.person_schedule_id,fact.household_id,fact.medication_take_id,fact.taken_at
FROM public.shared_schedule_adoption_facts fact
WHERE fact.person_schedule_id IS NOT NULL
 AND NOT EXISTS(SELECT 1 FROM public.shared_schedule_administrations existing
  WHERE existing.medication_take_id=fact.medication_take_id);

UPDATE public.shared_schedule_states state
SET latest_taken_at=latest.taken_at
FROM (SELECT entry.person_schedule_id,max(entry.taken_at) AS taken_at
 FROM public.shared_schedule_administrations entry GROUP BY entry.person_schedule_id) latest
WHERE state.id=latest.person_schedule_id
 AND (state.latest_taken_at IS NULL OR state.latest_taken_at<latest.taken_at);

ALTER TABLE public.shared_schedule_administrations ENABLE TRIGGER shared_administration_guard;
ALTER TABLE public.shared_schedule_adoption_facts ENABLE TRIGGER adoption_fact_guard;
