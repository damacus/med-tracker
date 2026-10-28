CREATE FUNCTION med_tracker.current_account_id() RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint;
$$;


ALTER FUNCTION med_tracker.current_account_id() OWNER TO med_tracker_owner;

CREATE FUNCTION med_tracker.current_household_id() RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_household_id', true), '')::bigint;
$$;


ALTER FUNCTION med_tracker.current_household_id() OWNER TO med_tracker_owner;

CREATE FUNCTION med_tracker.current_invitation_token_digest() RETURNS text
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_invitation_token_digest', true), '');
$$;


ALTER FUNCTION med_tracker.current_invitation_token_digest() OWNER TO med_tracker_owner;

CREATE FUNCTION med_tracker.current_membership_id() RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_membership_id', true), '')::bigint;
$$;


ALTER FUNCTION med_tracker.current_membership_id() OWNER TO med_tracker_owner;

CREATE FUNCTION med_tracker.purge_medication_takes(p_household_id bigint) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
DECLARE
  v_account_setting text := pg_catalog.current_setting('med_tracker.current_account_id', true);
  v_household_setting text := pg_catalog.current_setting('med_tracker.current_household_id', true);
  v_account_id bigint;
  v_household_id bigint;
  v_deleted_rows bigint;
BEGIN
  IF v_household_setting IS NULL
     OR v_household_setting !~ '^[0-9]+$' THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT104',
      MESSAGE = 'household purge tenant context does not match target';
  END IF;

  BEGIN
    v_household_id := v_household_setting::bigint;
  EXCEPTION
    WHEN numeric_value_out_of_range OR invalid_text_representation THEN
      RAISE EXCEPTION USING
        ERRCODE = 'MT104',
        MESSAGE = 'household purge tenant context does not match target';
  END;

  IF v_household_id IS DISTINCT FROM p_household_id THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT104',
      MESSAGE = 'household purge tenant context does not match target';
  END IF;

  IF v_account_setting IS NULL
     OR v_account_setting !~ '^[0-9]+$' THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT105',
      MESSAGE = 'household purge operator context is invalid';
  END IF;

  BEGIN
    v_account_id := v_account_setting::bigint;
  EXCEPTION
    WHEN numeric_value_out_of_range OR invalid_text_representation THEN
      RAISE EXCEPTION USING
        ERRCODE = 'MT105',
        MESSAGE = 'household purge operator context is invalid';
  END;

  IF NOT EXISTS (
       SELECT 1
       FROM public.platform_admins
       WHERE account_id = v_account_id
         AND status = 'active'
     ) THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT105',
      MESSAGE = 'household purge operator context is invalid';
  END IF;

  PERFORM 1
  FROM public.households
  WHERE id = p_household_id
  FOR UPDATE;

  IF NOT FOUND THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT101',
      MESSAGE = 'household purge target is invalid';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM public.households
    WHERE id = p_household_id
      AND status = 'archived'
      AND lifecycle_state = 'purging'
      AND offboarded_at IS NOT NULL
  ) THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT102',
      MESSAGE = 'household purge lifecycle is invalid';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM public.household_retention_holds
    WHERE household_id = p_household_id
      AND status = 'active'
  ) THEN
    RAISE EXCEPTION USING
      ERRCODE = 'MT103',
      MESSAGE = 'household purge retention hold is active';
  END IF;

  PERFORM pg_catalog.set_config(
    'med_tracker.current_household_id',
    p_household_id::text,
    true
  );
  PERFORM pg_catalog.set_config(
    'med_tracker.current_account_id',
    v_account_setting,
    true
  );

  DELETE FROM public.medication_takes
  WHERE household_id = p_household_id;

  GET DIAGNOSTICS v_deleted_rows = ROW_COUNT;

  PERFORM pg_catalog.set_config(
    'med_tracker.current_household_id',
    coalesce(v_household_setting, ''),
    true
  );
  PERFORM pg_catalog.set_config(
    'med_tracker.current_account_id',
    coalesce(v_account_setting, ''),
    true
  );

  RETURN v_deleted_rows;
EXCEPTION
  WHEN OTHERS THEN
    PERFORM pg_catalog.set_config(
      'med_tracker.current_household_id',
      coalesce(v_household_setting, ''),
      true
    );
    PERFORM pg_catalog.set_config(
      'med_tracker.current_account_id',
      coalesce(v_account_setting, ''),
      true
    );
    RAISE;
END;
$_$;


ALTER FUNCTION med_tracker.purge_medication_takes(p_household_id bigint) OWNER TO med_tracker_owner;

CREATE FUNCTION public.audit_append_ledger_entry(p_source_table text, p_source_id bigint, p_household_id bigint, p_source_payload jsonb, p_occurred_at timestamp with time zone) RETURNS void
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
    'outcome', v_metadata->>'outcome',
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


ALTER FUNCTION public.audit_append_ledger_entry(p_source_table text, p_source_id bigint, p_household_id bigint, p_source_payload jsonb, p_occurred_at timestamp with time zone) OWNER TO med_tracker_owner;

CREATE FUNCTION public.audit_capture_source_row() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
  PERFORM audit_append_ledger_entry(TG_TABLE_NAME, NEW.id, NEW.household_id, to_jsonb(NEW), NEW.created_at);
  RETURN NEW;
END;
$$;


ALTER FUNCTION public.audit_capture_source_row() OWNER TO med_tracker_owner;

CREATE FUNCTION public.audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
  v_entry audit_ledger_entries%ROWTYPE;
  v_key audit_signing_keys%ROWTYPE;
  v_checkpoint audit_checkpoints%ROWTYPE;
BEGIN
  SELECT * INTO STRICT v_entry
  FROM audit_ledger_entries
  WHERE chain_key = p_chain_key
    AND chain_epoch = p_chain_epoch
    AND sequence = p_sequence
    AND entry_hash = decode(p_entry_hash_hex, 'hex');

  INSERT INTO audit_signing_keys (key_id, algorithm, public_key, active_from, created_at, updated_at)
  VALUES (p_key_id, 'ed25519', decode(p_public_key_base64, 'base64'), p_signed_at,
          clock_timestamp(), clock_timestamp())
  ON CONFLICT (key_id) DO NOTHING;

  SELECT * INTO STRICT v_key FROM audit_signing_keys WHERE key_id = p_key_id;
  IF v_key.public_key <> decode(p_public_key_base64, 'base64') THEN
    RAISE EXCEPTION 'audit signing key id is already registered with different key material';
  END IF;

  SELECT * INTO v_checkpoint
  FROM audit_checkpoints
  WHERE chain_key = p_chain_key AND chain_epoch = p_chain_epoch AND sequence = p_sequence
  FOR UPDATE;

  IF FOUND AND v_checkpoint.signature IS NOT NULL THEN
    RAISE EXCEPTION 'audit checkpoint is already signed';
  END IF;

  IF FOUND THEN
    UPDATE audit_checkpoints
    SET audit_signing_key_id = v_key.id,
        signature = decode(p_signature_base64, 'base64'),
        signed_at = p_signed_at,
        updated_at = clock_timestamp()
    WHERE id = v_checkpoint.id
    RETURNING * INTO v_checkpoint;
  ELSE
    INSERT INTO audit_checkpoints (
      household_id, audit_signing_key_id, chain_key, chain_epoch, checkpoint_kind,
      sequence, entry_hash, signature, signed_at, created_at, updated_at
    ) VALUES (
      v_entry.household_id, v_key.id, p_chain_key, p_chain_epoch, p_checkpoint_kind,
      p_sequence, decode(p_entry_hash_hex, 'hex'), decode(p_signature_base64, 'base64'),
      p_signed_at, clock_timestamp(), clock_timestamp()
    ) RETURNING * INTO v_checkpoint;
  END IF;

  INSERT INTO audit_export_deliveries (
    audit_checkpoint_id, status, attempts, created_at, updated_at
  ) VALUES (
    v_checkpoint.id, 'pending', 0, clock_timestamp(), clock_timestamp()
  ) ON CONFLICT (audit_checkpoint_id) DO NOTHING;

  RETURN v_checkpoint.id;
END;
$$;


ALTER FUNCTION public.audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text) OWNER TO med_tracker_owner;
