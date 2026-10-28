CREATE OR REPLACE FUNCTION public.audit_append_ledger_entry(p_source_table text, p_source_id bigint, p_household_id bigint, p_source_payload jsonb, p_occurred_at timestamp with time zone) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
  v_chain_key text := COALESCE('household:' || p_household_id::text, 'global');
  v_head audit_chain_heads%ROWTYPE;
  v_sequence bigint;
  v_context jsonb := COALESCE(p_source_payload->'audit_context', '{}'::jsonb);
  v_metadata jsonb := COALESCE(p_source_payload->'metadata', '{}'::jsonb);
  v_occurred_at timestamptz := COALESCE(p_occurred_at, clock_timestamp());
  v_retain_until timestamptz := v_occurred_at + interval '10 years';
  v_policy_version text := COALESCE(v_context->>'retention_policy_version', 'clinical-security-v1');
  v_envelope jsonb;
  v_canonical_payload bytea;
  v_entry_hash bytea;
BEGIN
  INSERT INTO audit_chain_heads (chain_key, household_id, created_at, updated_at)
  VALUES (v_chain_key, p_household_id, clock_timestamp(), clock_timestamp())
  ON CONFLICT (chain_key) DO NOTHING;

  SELECT * INTO STRICT v_head
  FROM audit_chain_heads
  WHERE chain_key = v_chain_key
  FOR UPDATE;

  v_sequence := v_head.last_sequence + 1;
  v_envelope := jsonb_strip_nulls(jsonb_build_object(
    'schema_version', 1,
    'event_id', gen_random_uuid(),
    'event_type', COALESCE(p_source_payload->>'event_type', p_source_payload->>'event'),
    'outcome', CASE WHEN p_source_table = 'versions'
      AND p_source_payload->>'item_type' = 'AccountClinicalAccess'
      THEN v_context->>'outcome'
      ELSE v_metadata->>'outcome' END,
    'occurred_at', v_occurred_at,
    'household_id', p_household_id,
    'agent', jsonb_build_object(
      'account_id', v_context->'actor_account_id',
      'person_id', v_context->'actor_person_id',
      'membership_id', v_context->'actor_membership_id',
      'role', v_context->'active_role',
      'permissions_version', v_context->'permissions_version',
      'authentication_method', v_context->'authentication_method',
      'session_reference', v_context->'session_reference'
    ),
    'policy', jsonb_build_object('class', v_context->'policy_class', 'query', v_context->'policy_query'),
    'request', jsonb_build_object(
      'request_id', v_context->'request_id',
      'ip', v_context->'ip',
      'support_access_session_id', v_context->'support_access_session_id'
    ),
    'source', jsonb_build_object('table', p_source_table, 'id', p_source_id),
    'entity', CASE WHEN p_source_table = 'versions' THEN
      jsonb_build_object('type', p_source_payload->'item_type', 'id', p_source_payload->'item_id')
    ELSE v_metadata - 'outcome' END,
    'retention', jsonb_build_object('policy_version', v_policy_version, 'retain_until', v_retain_until)
  ));
  v_canonical_payload := convert_to(v_envelope::text, 'UTF8');
  v_entry_hash := digest(
    convert_to(
      'medtracker.audit.ledger.v1' || chr(31) || v_chain_key || chr(31) ||
      v_head.chain_epoch::text || chr(31) || v_sequence::text || chr(31),
      'UTF8'
    ) || COALESCE(v_head.last_hash, '\x'::bytea) || v_canonical_payload,
    'sha256'
  );

  INSERT INTO audit_ledger_entries (
    household_id, chain_key, chain_epoch, epoch_kind, sequence, previous_hash, entry_hash,
    source_table, source_id, source_payload, envelope, canonical_payload, occurred_at,
    retention_policy_version, retain_until, created_at, updated_at
  ) VALUES (
    p_household_id, v_chain_key, v_head.chain_epoch, v_head.epoch_kind, v_sequence, v_head.last_hash,
    v_entry_hash, p_source_table, p_source_id, p_source_payload - 'updated_at', v_envelope,
    v_canonical_payload, v_occurred_at, v_policy_version, v_retain_until, clock_timestamp(), clock_timestamp()
  );

  INSERT INTO audit_export_deliveries (audit_ledger_entry_id, status, attempts, created_at, updated_at)
  VALUES (currval(pg_get_serial_sequence('audit_ledger_entries', 'id')), 'pending', 0,
          clock_timestamp(), clock_timestamp());

  UPDATE audit_chain_heads
  SET last_sequence = v_sequence, last_hash = v_entry_hash, updated_at = clock_timestamp()
  WHERE id = v_head.id;
END;
$$;
