-- MedTracker PostgreSQL schema baseline.
--
-- Provisioning artifact for environments that must not depend on the Rails
-- application to stand up the database. Generated from a fully migrated
-- reference database (all 177 Rails migrations applied, including the
-- tamper-evident audit ledger objects that db/schema.rb cannot express).
--
-- Regenerate from a fully migrated database after any Rails migration change:
--   pg_dump --schema-only -d <db> | grep -v -e "^\\restrict" -e "^\\unrestrict" > db/schema.sql
--   pg_dump --data-only -t schema_migrations -t ar_internal_metadata -d <db> | grep -v -e "^\\restrict" -e "^\\unrestrict" >> db/schema.sql
-- Before Rails retirement, regenerate once from the production database.

--
-- PostgreSQL database dump
--


-- Dumped from database version 18.6
-- Dumped by pg_dump version 18.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: med_tracker; Type: SCHEMA; Schema: -; Owner: medtracker
--

CREATE SCHEMA med_tracker;


ALTER SCHEMA med_tracker OWNER TO medtracker;

--
-- Name: citext; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS citext WITH SCHEMA public;


--
-- Name: EXTENSION citext; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION citext IS 'data type for case-insensitive character strings';


--
-- Name: pg_trgm; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;


--
-- Name: EXTENSION pg_trgm; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pg_trgm IS 'text similarity measurement and index searching based on trigrams';


--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: current_account_id(); Type: FUNCTION; Schema: med_tracker; Owner: med_tracker_owner
--

CREATE FUNCTION med_tracker.current_account_id() RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint;
$$;


ALTER FUNCTION med_tracker.current_account_id() OWNER TO med_tracker_owner;

--
-- Name: current_household_id(); Type: FUNCTION; Schema: med_tracker; Owner: med_tracker_owner
--

CREATE FUNCTION med_tracker.current_household_id() RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_household_id', true), '')::bigint;
$$;


ALTER FUNCTION med_tracker.current_household_id() OWNER TO med_tracker_owner;

--
-- Name: current_invitation_token_digest(); Type: FUNCTION; Schema: med_tracker; Owner: med_tracker_owner
--

CREATE FUNCTION med_tracker.current_invitation_token_digest() RETURNS text
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_invitation_token_digest', true), '');
$$;


ALTER FUNCTION med_tracker.current_invitation_token_digest() OWNER TO med_tracker_owner;

--
-- Name: current_membership_id(); Type: FUNCTION; Schema: med_tracker; Owner: med_tracker_owner
--

CREATE FUNCTION med_tracker.current_membership_id() RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT NULLIF(current_setting('med_tracker.current_membership_id', true), '')::bigint;
$$;


ALTER FUNCTION med_tracker.current_membership_id() OWNER TO med_tracker_owner;

--
-- Name: purge_medication_takes(bigint); Type: FUNCTION; Schema: med_tracker; Owner: med_tracker_owner
--

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

--
-- Name: audit_append_ledger_entry(text, bigint, bigint, jsonb, timestamp with time zone); Type: FUNCTION; Schema: public; Owner: med_tracker_owner
--

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
      'user_id', v_context->'actor_user_id',
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

--
-- Name: audit_capture_source_row(); Type: FUNCTION; Schema: public; Owner: med_tracker_owner
--

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

--
-- Name: audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamp with time zone, text); Type: FUNCTION; Schema: public; Owner: med_tracker_owner
--

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

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: account_active_session_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_active_session_keys (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_use timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    session_id character varying NOT NULL
);


ALTER TABLE public.account_active_session_keys OWNER TO med_tracker_owner;

--
-- Name: account_identities; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_identities (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    provider character varying NOT NULL,
    uid character varying NOT NULL,
    updated_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


ALTER TABLE public.account_identities OWNER TO med_tracker_owner;

--
-- Name: account_identities_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_identities_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_identities_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_identities_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_identities_id_seq OWNED BY public.account_identities.id;


--
-- Name: account_lockouts; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_lockouts (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone,
    deadline timestamp(6) without time zone NOT NULL,
    email_last_sent timestamp(6) without time zone,
    key character varying NOT NULL,
    updated_at timestamp(6) without time zone
);


ALTER TABLE public.account_lockouts OWNER TO med_tracker_owner;

--
-- Name: account_lockouts_account_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_lockouts_account_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_lockouts_account_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_lockouts_account_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_lockouts_account_id_seq OWNED BY public.account_lockouts.account_id;


--
-- Name: account_login_change_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_login_change_keys (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone,
    deadline timestamp(6) without time zone NOT NULL,
    key character varying NOT NULL,
    login character varying NOT NULL,
    updated_at timestamp(6) without time zone
);


ALTER TABLE public.account_login_change_keys OWNER TO med_tracker_owner;

--
-- Name: account_login_failures; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_login_failures (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone,
    number integer DEFAULT 1 NOT NULL,
    updated_at timestamp(6) without time zone
);


ALTER TABLE public.account_login_failures OWNER TO med_tracker_owner;

--
-- Name: account_login_failures_account_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_login_failures_account_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_login_failures_account_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_login_failures_account_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_login_failures_account_id_seq OWNED BY public.account_login_failures.account_id;


--
-- Name: account_otp_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_otp_keys (
    id bigint NOT NULL,
    key character varying NOT NULL,
    last_use timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    num_failures integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.account_otp_keys OWNER TO med_tracker_owner;

--
-- Name: account_otp_keys_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_otp_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_otp_keys_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_otp_keys_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_otp_keys_id_seq OWNED BY public.account_otp_keys.id;


--
-- Name: account_password_reset_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_password_reset_keys (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone,
    deadline timestamp(6) without time zone NOT NULL,
    email_last_sent timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    key character varying NOT NULL,
    updated_at timestamp(6) without time zone
);


ALTER TABLE public.account_password_reset_keys OWNER TO med_tracker_owner;

--
-- Name: account_recovery_codes; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_recovery_codes (
    code character varying NOT NULL,
    id bigint NOT NULL
);


ALTER TABLE public.account_recovery_codes OWNER TO med_tracker_owner;

--
-- Name: account_remember_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_remember_keys (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone,
    deadline timestamp(6) without time zone NOT NULL,
    key character varying NOT NULL,
    updated_at timestamp(6) without time zone
);


ALTER TABLE public.account_remember_keys OWNER TO med_tracker_owner;

--
-- Name: account_verification_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_verification_keys (
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone,
    email_last_sent timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    key character varying NOT NULL,
    requested_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp(6) without time zone
);


ALTER TABLE public.account_verification_keys OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_auth_challenges; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_webauthn_auth_challenges (
    id bigint NOT NULL,
    challenge_digest character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.account_webauthn_auth_challenges OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_auth_challenges_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_webauthn_auth_challenges_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_webauthn_auth_challenges_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_auth_challenges_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_webauthn_auth_challenges_id_seq OWNED BY public.account_webauthn_auth_challenges.id;


--
-- Name: account_webauthn_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_webauthn_keys (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_use timestamp(6) without time zone,
    nickname character varying,
    public_key character varying NOT NULL,
    sign_count integer DEFAULT 0 NOT NULL,
    updated_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    webauthn_id character varying NOT NULL
);


ALTER TABLE public.account_webauthn_keys OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_keys_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_webauthn_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_webauthn_keys_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_keys_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_webauthn_keys_id_seq OWNED BY public.account_webauthn_keys.id;


--
-- Name: account_webauthn_user_ids; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.account_webauthn_user_ids (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp(6) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    webauthn_id character varying NOT NULL
);


ALTER TABLE public.account_webauthn_user_ids OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_user_ids_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.account_webauthn_user_ids_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_webauthn_user_ids_id_seq OWNER TO med_tracker_owner;

--
-- Name: account_webauthn_user_ids_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.account_webauthn_user_ids_id_seq OWNED BY public.account_webauthn_user_ids.id;


--
-- Name: accounts; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.accounts (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    email public.citext NOT NULL,
    password_hash character varying,
    preferences jsonb DEFAULT '{}'::jsonb NOT NULL,
    status integer DEFAULT 1 NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.accounts OWNER TO med_tracker_owner;

--
-- Name: accounts_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.accounts_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.accounts_id_seq OWNER TO med_tracker_owner;

--
-- Name: accounts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.accounts_id_seq OWNED BY public.accounts.id;


--
-- Name: active_storage_attachments; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.active_storage_attachments (
    id bigint NOT NULL,
    blob_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    name character varying NOT NULL,
    record_id bigint NOT NULL,
    record_type character varying NOT NULL
);

ALTER TABLE ONLY public.active_storage_attachments FORCE ROW LEVEL SECURITY;


ALTER TABLE public.active_storage_attachments OWNER TO med_tracker_owner;

--
-- Name: active_storage_attachments_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.active_storage_attachments_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.active_storage_attachments_id_seq OWNER TO med_tracker_owner;

--
-- Name: active_storage_attachments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.active_storage_attachments_id_seq OWNED BY public.active_storage_attachments.id;


--
-- Name: active_storage_blobs; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.active_storage_blobs (
    id bigint NOT NULL,
    byte_size bigint NOT NULL,
    checksum character varying,
    content_type character varying,
    created_at timestamp(6) without time zone NOT NULL,
    filename character varying NOT NULL,
    key character varying NOT NULL,
    metadata text,
    service_name character varying NOT NULL
);


ALTER TABLE public.active_storage_blobs OWNER TO med_tracker_owner;

--
-- Name: active_storage_blobs_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.active_storage_blobs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.active_storage_blobs_id_seq OWNER TO med_tracker_owner;

--
-- Name: active_storage_blobs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.active_storage_blobs_id_seq OWNED BY public.active_storage_blobs.id;


--
-- Name: active_storage_variant_records; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.active_storage_variant_records (
    id bigint NOT NULL,
    blob_id bigint NOT NULL,
    variation_digest character varying NOT NULL
);


ALTER TABLE public.active_storage_variant_records OWNER TO med_tracker_owner;

--
-- Name: active_storage_variant_records_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.active_storage_variant_records_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.active_storage_variant_records_id_seq OWNER TO med_tracker_owner;

--
-- Name: active_storage_variant_records_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.active_storage_variant_records_id_seq OWNED BY public.active_storage_variant_records.id;


--
-- Name: api_app_tokens; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.api_app_tokens (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    expires_at timestamp(6) without time zone NOT NULL,
    household_membership_id bigint NOT NULL,
    last_used_at timestamp(6) without time zone NOT NULL,
    name character varying NOT NULL,
    permissions_version integer DEFAULT 1 NOT NULL,
    revoked_at timestamp(6) without time zone,
    token_digest character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT api_app_token_positive_lifetime CHECK ((expires_at > created_at))
);


ALTER TABLE public.api_app_tokens OWNER TO med_tracker_owner;

--
-- Name: api_app_tokens_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.api_app_tokens_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_app_tokens_id_seq OWNER TO med_tracker_owner;

--
-- Name: api_app_tokens_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.api_app_tokens_id_seq OWNED BY public.api_app_tokens.id;


--
-- Name: api_change_events; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.api_change_events (
    id bigint NOT NULL,
    action character varying NOT NULL,
    account_id bigint,
    created_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    household_membership_id bigint,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    occurred_at timestamp(6) without time zone NOT NULL,
    record_portable_id character varying,
    record_id bigint NOT NULL,
    record_type character varying NOT NULL,
    request_id character varying,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.api_change_events FORCE ROW LEVEL SECURITY;


ALTER TABLE public.api_change_events OWNER TO med_tracker_owner;

--
-- Name: api_change_events_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.api_change_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_change_events_id_seq OWNER TO med_tracker_owner;

--
-- Name: api_change_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.api_change_events_id_seq OWNED BY public.api_change_events.id;


--
-- Name: api_household_selection_grants; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.api_household_selection_grants (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    device_name character varying,
    expires_at timestamp(6) without time zone NOT NULL,
    mfa_verified_at timestamp(6) without time zone,
    oidc_mfa_verified boolean DEFAULT false NOT NULL,
    token_digest character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    used_at timestamp(6) without time zone,
    user_agent character varying
);


ALTER TABLE public.api_household_selection_grants OWNER TO med_tracker_owner;

--
-- Name: api_household_selection_grants_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.api_household_selection_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_household_selection_grants_id_seq OWNER TO med_tracker_owner;

--
-- Name: api_household_selection_grants_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.api_household_selection_grants_id_seq OWNED BY public.api_household_selection_grants.id;


--
-- Name: api_idempotency_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.api_idempotency_keys (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    api_app_token_id bigint,
    api_session_id bigint,
    created_at timestamp(6) without time zone NOT NULL,
    expires_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    key character varying NOT NULL,
    request_digest character varying NOT NULL,
    request_method character varying NOT NULL,
    request_path character varying NOT NULL,
    response_body jsonb DEFAULT '{}'::jsonb NOT NULL,
    response_headers jsonb DEFAULT '{}'::jsonb NOT NULL,
    response_status integer NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.api_idempotency_keys FORCE ROW LEVEL SECURITY;


ALTER TABLE public.api_idempotency_keys OWNER TO med_tracker_owner;

--
-- Name: api_idempotency_keys_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.api_idempotency_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_idempotency_keys_id_seq OWNER TO med_tracker_owner;

--
-- Name: api_idempotency_keys_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.api_idempotency_keys_id_seq OWNED BY public.api_idempotency_keys.id;


--
-- Name: api_sessions; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.api_sessions (
    id bigint NOT NULL,
    access_expires_at timestamp(6) without time zone NOT NULL,
    access_token_digest character varying NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    device_name character varying,
    household_membership_id bigint,
    last_used_at timestamp(6) without time zone NOT NULL,
    mfa_verified_at timestamp(6) without time zone,
    oidc_mfa_verified boolean DEFAULT false NOT NULL,
    permissions_version integer DEFAULT 1 NOT NULL,
    refresh_expires_at timestamp(6) without time zone NOT NULL,
    refresh_token_digest character varying NOT NULL,
    revoked_at timestamp(6) without time zone,
    updated_at timestamp(6) without time zone NOT NULL,
    user_agent character varying
);


ALTER TABLE public.api_sessions OWNER TO med_tracker_owner;

--
-- Name: api_sessions_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.api_sessions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_sessions_id_seq OWNER TO med_tracker_owner;

--
-- Name: api_sessions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.api_sessions_id_seq OWNED BY public.api_sessions.id;


--
-- Name: api_tombstones; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.api_tombstones (
    id bigint NOT NULL,
    action character varying DEFAULT 'delete'::character varying NOT NULL,
    account_id bigint,
    created_at timestamp(6) without time zone NOT NULL,
    deleted_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    household_membership_id bigint,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    record_portable_id character varying NOT NULL,
    record_type character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.api_tombstones FORCE ROW LEVEL SECURITY;


ALTER TABLE public.api_tombstones OWNER TO med_tracker_owner;

--
-- Name: api_tombstones_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.api_tombstones_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_tombstones_id_seq OWNER TO med_tracker_owner;

--
-- Name: api_tombstones_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.api_tombstones_id_seq OWNED BY public.api_tombstones.id;


--
-- Name: app_settings; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.app_settings (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    invite_only boolean DEFAULT false NOT NULL,
    medicine_lookup_base_url character varying DEFAULT 'https://ontology.nhs.uk/production1/fhir'::character varying NOT NULL,
    medicine_lookup_source_priority jsonb DEFAULT '["imported_catalog", "local_nhs_dmd", "cached_open_products_facts", "open_products_facts", "curated_catalog", "nhs_dmd", "supplements"]'::jsonb NOT NULL,
    medicine_lookup_token_url character varying DEFAULT 'https://ontology.nhs.uk/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.app_settings OWNER TO med_tracker_owner;

--
-- Name: app_settings_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.app_settings_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.app_settings_id_seq OWNER TO med_tracker_owner;

--
-- Name: app_settings_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.app_settings_id_seq OWNED BY public.app_settings.id;


--
-- Name: ar_internal_metadata; Type: TABLE; Schema: public; Owner: medtracker
--

CREATE TABLE public.ar_internal_metadata (
    key character varying NOT NULL,
    value character varying,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.ar_internal_metadata OWNER TO medtracker;

--
-- Name: audit_chain_heads; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.audit_chain_heads (
    id bigint NOT NULL,
    chain_epoch uuid DEFAULT gen_random_uuid() NOT NULL,
    chain_key character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    epoch_kind character varying DEFAULT 'live'::character varying NOT NULL,
    household_id bigint,
    last_hash bytea,
    last_sequence bigint DEFAULT 0 NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT audit_chain_heads_last_hash_length CHECK (((last_hash IS NULL) OR (octet_length(last_hash) = 32)))
);


ALTER TABLE public.audit_chain_heads OWNER TO med_tracker_owner;

--
-- Name: audit_chain_heads_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.audit_chain_heads_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_chain_heads_id_seq OWNER TO med_tracker_owner;

--
-- Name: audit_chain_heads_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.audit_chain_heads_id_seq OWNED BY public.audit_chain_heads.id;


--
-- Name: audit_checkpoints; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.audit_checkpoints (
    id bigint NOT NULL,
    audit_signing_key_id bigint,
    chain_epoch uuid NOT NULL,
    chain_key character varying NOT NULL,
    checkpoint_kind character varying DEFAULT 'periodic'::character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    entry_hash bytea NOT NULL,
    household_id bigint,
    sequence bigint NOT NULL,
    signature bytea,
    signed_at timestamp(6) without time zone,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT audit_checkpoint_entry_hash_length CHECK ((octet_length(entry_hash) = 32))
);


ALTER TABLE public.audit_checkpoints OWNER TO med_tracker_owner;

--
-- Name: audit_checkpoints_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.audit_checkpoints_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_checkpoints_id_seq OWNER TO med_tracker_owner;

--
-- Name: audit_checkpoints_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.audit_checkpoints_id_seq OWNED BY public.audit_checkpoints.id;


--
-- Name: audit_export_deliveries; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.audit_export_deliveries (
    id bigint NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    audit_checkpoint_id bigint,
    audit_ledger_entry_id bigint,
    checksum_sha256 character varying,
    created_at timestamp(6) without time zone NOT NULL,
    delivered_at timestamp(6) without time zone,
    last_error_code character varying,
    last_error_message text,
    next_attempt_at timestamp(6) without time zone,
    object_key character varying,
    object_version_id character varying,
    retain_until timestamp(6) without time zone,
    retention_mode character varying,
    status character varying DEFAULT 'pending'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT audit_export_delivery_exactly_one_record CHECK (((audit_ledger_entry_id IS NULL) <> (audit_checkpoint_id IS NULL)))
);


ALTER TABLE public.audit_export_deliveries OWNER TO med_tracker_owner;

--
-- Name: audit_export_deliveries_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.audit_export_deliveries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_export_deliveries_id_seq OWNER TO med_tracker_owner;

--
-- Name: audit_export_deliveries_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.audit_export_deliveries_id_seq OWNED BY public.audit_export_deliveries.id;


--
-- Name: audit_ledger_entries; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.audit_ledger_entries (
    id bigint NOT NULL,
    canonical_payload bytea NOT NULL,
    chain_epoch uuid NOT NULL,
    chain_key character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    entry_hash bytea NOT NULL,
    envelope jsonb NOT NULL,
    epoch_kind character varying NOT NULL,
    hash_algorithm character varying DEFAULT 'sha256'::character varying NOT NULL,
    household_id bigint,
    occurred_at timestamp(6) without time zone NOT NULL,
    previous_hash bytea,
    retain_until timestamp(6) without time zone NOT NULL,
    retention_policy_version character varying NOT NULL,
    schema_version integer DEFAULT 1 NOT NULL,
    sequence bigint NOT NULL,
    source_id bigint NOT NULL,
    source_payload jsonb NOT NULL,
    source_table character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT audit_ledger_entry_hash_length CHECK ((octet_length(entry_hash) = 32)),
    CONSTRAINT audit_ledger_positive_sequence CHECK ((sequence > 0)),
    CONSTRAINT audit_ledger_previous_hash_length CHECK (((previous_hash IS NULL) OR (octet_length(previous_hash) = 32))),
    CONSTRAINT audit_ledger_retention_after_event CHECK ((retain_until >= occurred_at))
);


ALTER TABLE public.audit_ledger_entries OWNER TO med_tracker_owner;

--
-- Name: audit_ledger_entries_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.audit_ledger_entries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_ledger_entries_id_seq OWNER TO med_tracker_owner;

--
-- Name: audit_ledger_entries_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.audit_ledger_entries_id_seq OWNED BY public.audit_ledger_entries.id;


--
-- Name: audit_signing_keys; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.audit_signing_keys (
    id bigint NOT NULL,
    active_from timestamp(6) without time zone NOT NULL,
    algorithm character varying DEFAULT 'ed25519'::character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    key_id character varying NOT NULL,
    public_key bytea NOT NULL,
    retired_at timestamp(6) without time zone,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.audit_signing_keys OWNER TO med_tracker_owner;

--
-- Name: audit_signing_keys_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.audit_signing_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_signing_keys_id_seq OWNER TO med_tracker_owner;

--
-- Name: audit_signing_keys_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.audit_signing_keys_id_seq OWNED BY public.audit_signing_keys.id;


--
-- Name: barcode_catalog_entries; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.barcode_catalog_entries (
    id bigint NOT NULL,
    code character varying,
    concept_class character varying,
    created_at timestamp(6) without time zone NOT NULL,
    display character varying NOT NULL,
    gtin character varying NOT NULL,
    source character varying NOT NULL,
    system character varying,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.barcode_catalog_entries OWNER TO med_tracker_owner;

--
-- Name: barcode_catalog_entries_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.barcode_catalog_entries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.barcode_catalog_entries_id_seq OWNER TO med_tracker_owner;

--
-- Name: barcode_catalog_entries_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.barcode_catalog_entries_id_seq OWNED BY public.barcode_catalog_entries.id;


--
-- Name: carer_relationships; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.carer_relationships (
    id bigint NOT NULL,
    active boolean DEFAULT true NOT NULL,
    carer_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    patient_id bigint NOT NULL,
    relationship_type character varying,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.carer_relationships FORCE ROW LEVEL SECURITY;


ALTER TABLE public.carer_relationships OWNER TO med_tracker_owner;

--
-- Name: carer_relationships_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.carer_relationships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.carer_relationships_id_seq OWNER TO med_tracker_owner;

--
-- Name: carer_relationships_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.carer_relationships_id_seq OWNED BY public.carer_relationships.id;


--
-- Name: dosages; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.dosages (
    id bigint NOT NULL,
    amount numeric NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    current_supply numeric(10,2),
    default_dose_cycle integer NOT NULL,
    default_for_adults boolean DEFAULT false NOT NULL,
    default_for_children boolean DEFAULT false NOT NULL,
    default_max_daily_doses integer NOT NULL,
    default_min_hours_between_doses numeric(4,1) NOT NULL,
    description character varying,
    frequency character varying NOT NULL,
    household_id bigint NOT NULL,
    medication_id bigint NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    reorder_threshold numeric(10,2),
    unit character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.dosages FORCE ROW LEVEL SECURITY;


ALTER TABLE public.dosages OWNER TO med_tracker_owner;

--
-- Name: dosages_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.dosages_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.dosages_id_seq OWNER TO med_tracker_owner;

--
-- Name: dosages_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.dosages_id_seq OWNED BY public.dosages.id;


--
-- Name: health_event_medications; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.health_event_medications (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    health_event_id bigint NOT NULL,
    household_id bigint NOT NULL,
    medication_id bigint,
    medication_name character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.health_event_medications FORCE ROW LEVEL SECURITY;


ALTER TABLE public.health_event_medications OWNER TO med_tracker_owner;

--
-- Name: health_event_medications_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.health_event_medications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.health_event_medications_id_seq OWNER TO med_tracker_owner;

--
-- Name: health_event_medications_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.health_event_medications_id_seq OWNED BY public.health_event_medications.id;


--
-- Name: health_events; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.health_events (
    id bigint NOT NULL,
    action_taken text,
    created_at timestamp(6) without time zone NOT NULL,
    ended_on date,
    event_kind integer NOT NULL,
    household_id bigint NOT NULL,
    medical_help_sought boolean DEFAULT false NOT NULL,
    notes text,
    person_id bigint NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    severity integer,
    started_on date NOT NULL,
    title character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.health_events FORCE ROW LEVEL SECURITY;


ALTER TABLE public.health_events OWNER TO med_tracker_owner;

--
-- Name: health_events_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.health_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.health_events_id_seq OWNER TO med_tracker_owner;

--
-- Name: health_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.health_events_id_seq OWNED BY public.health_events.id;


--
-- Name: household_audit_ledger_entries; Type: VIEW; Schema: public; Owner: med_tracker_owner
--

CREATE VIEW public.household_audit_ledger_entries WITH (security_barrier='true') AS
 SELECT id,
    household_id,
    chain_key,
    chain_epoch,
    epoch_kind,
    sequence,
    previous_hash,
    entry_hash,
    hash_algorithm,
    schema_version,
    source_table,
    source_id,
    envelope,
    occurred_at,
    retention_policy_version,
    retain_until,
    created_at
   FROM public.audit_ledger_entries
  WHERE (household_id = med_tracker.current_household_id());


ALTER VIEW public.household_audit_ledger_entries OWNER TO med_tracker_owner;

--
-- Name: household_exports; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.household_exports (
    id bigint NOT NULL,
    artifact_byte_size bigint,
    artifact_checksum_sha256 character varying,
    created_at timestamp(6) without time zone NOT NULL,
    downloaded_at timestamp(6) without time zone,
    expired_at timestamp(6) without time zone,
    expires_at timestamp(6) without time zone,
    failed_at timestamp(6) without time zone,
    failure_code character varying,
    generation_started_at timestamp(6) without time zone,
    household_id bigint NOT NULL,
    manifest jsonb DEFAULT '{}'::jsonb NOT NULL,
    ready_at timestamp(6) without time zone,
    requested_at timestamp(6) without time zone NOT NULL,
    requested_by_account_id bigint NOT NULL,
    status character varying DEFAULT 'requested'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.household_exports FORCE ROW LEVEL SECURITY;


ALTER TABLE public.household_exports OWNER TO med_tracker_owner;

--
-- Name: household_exports_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.household_exports_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_exports_id_seq OWNER TO med_tracker_owner;

--
-- Name: household_exports_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.household_exports_id_seq OWNED BY public.household_exports.id;


--
-- Name: household_invitation_grants; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.household_invitation_grants (
    id bigint NOT NULL,
    access_level character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    expires_at timestamp(6) without time zone,
    household_id bigint NOT NULL,
    household_invitation_id bigint NOT NULL,
    person_id bigint NOT NULL,
    relationship_type character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.household_invitation_grants FORCE ROW LEVEL SECURITY;


ALTER TABLE public.household_invitation_grants OWNER TO med_tracker_owner;

--
-- Name: household_invitation_grants_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.household_invitation_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_invitation_grants_id_seq OWNER TO med_tracker_owner;

--
-- Name: household_invitation_grants_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.household_invitation_grants_id_seq OWNED BY public.household_invitation_grants.id;


--
-- Name: household_invitations; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.household_invitations (
    id bigint NOT NULL,
    accepted_at timestamp(6) without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    email public.citext NOT NULL,
    expires_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    invited_by_membership_id bigint NOT NULL,
    membership_role character varying NOT NULL,
    revoked_at timestamp(6) without time zone,
    token_digest character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.household_invitations FORCE ROW LEVEL SECURITY;


ALTER TABLE public.household_invitations OWNER TO med_tracker_owner;

--
-- Name: household_invitations_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.household_invitations_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_invitations_id_seq OWNER TO med_tracker_owner;

--
-- Name: household_invitations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.household_invitations_id_seq OWNED BY public.household_invitations.id;


--
-- Name: household_memberships; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.household_memberships (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    joined_at timestamp(6) without time zone NOT NULL,
    permissions_version integer DEFAULT 1 NOT NULL,
    person_id bigint,
    revoked_at timestamp(6) without time zone,
    role character varying DEFAULT 'member'::character varying NOT NULL,
    status character varying DEFAULT 'active'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.household_memberships FORCE ROW LEVEL SECURITY;


ALTER TABLE public.household_memberships OWNER TO med_tracker_owner;

--
-- Name: household_memberships_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.household_memberships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_memberships_id_seq OWNER TO med_tracker_owner;

--
-- Name: household_memberships_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.household_memberships_id_seq OWNED BY public.household_memberships.id;


--
-- Name: household_purge_runs; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.household_purge_runs (
    id bigint NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    completed_at timestamp(6) without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    failed_at timestamp(6) without time zone,
    failure_code character varying,
    household_id bigint NOT NULL,
    last_completed_table character varying,
    requested_by_account_id bigint NOT NULL,
    started_at timestamp(6) without time zone,
    status character varying DEFAULT 'pending'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.household_purge_runs OWNER TO med_tracker_owner;

--
-- Name: household_purge_runs_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.household_purge_runs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_purge_runs_id_seq OWNER TO med_tracker_owner;

--
-- Name: household_purge_runs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.household_purge_runs_id_seq OWNED BY public.household_purge_runs.id;


--
-- Name: household_retention_holds; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.household_retention_holds (
    id bigint NOT NULL,
    approved_by_account_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    placed_at timestamp(6) without time zone NOT NULL,
    reason text NOT NULL,
    released_at timestamp(6) without time zone,
    released_by_account_id bigint,
    review_on date NOT NULL,
    status character varying DEFAULT 'active'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.household_retention_holds FORCE ROW LEVEL SECURITY;


ALTER TABLE public.household_retention_holds OWNER TO med_tracker_owner;

--
-- Name: household_retention_holds_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.household_retention_holds_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_retention_holds_id_seq OWNER TO med_tracker_owner;

--
-- Name: household_retention_holds_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.household_retention_holds_id_seq OWNED BY public.household_retention_holds.id;


--
-- Name: households; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.households (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    created_by_account_id bigint,
    lifecycle_state character varying DEFAULT 'active'::character varying NOT NULL,
    name character varying NOT NULL,
    offboarded_at timestamp(6) without time zone,
    slug character varying NOT NULL,
    status character varying DEFAULT 'active'::character varying NOT NULL,
    subscription_plan character varying DEFAULT 'free'::character varying NOT NULL,
    timezone character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.households OWNER TO med_tracker_owner;

--
-- Name: households_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.households_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.households_id_seq OWNER TO med_tracker_owner;

--
-- Name: households_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.households_id_seq OWNED BY public.households.id;


--
-- Name: location_memberships; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.location_memberships (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    location_id bigint NOT NULL,
    person_id bigint NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.location_memberships FORCE ROW LEVEL SECURITY;


ALTER TABLE public.location_memberships OWNER TO med_tracker_owner;

--
-- Name: location_memberships_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.location_memberships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.location_memberships_id_seq OWNER TO med_tracker_owner;

--
-- Name: location_memberships_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.location_memberships_id_seq OWNED BY public.location_memberships.id;


--
-- Name: locations; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.locations (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    description text,
    household_id bigint NOT NULL,
    name character varying NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.locations FORCE ROW LEVEL SECURITY;


ALTER TABLE public.locations OWNER TO med_tracker_owner;

--
-- Name: locations_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.locations_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.locations_id_seq OWNER TO med_tracker_owner;

--
-- Name: locations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.locations_id_seq OWNED BY public.locations.id;


--
-- Name: medication_dose_occurrences; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medication_dose_occurrences (
    id bigint NOT NULL,
    household_id bigint NOT NULL,
    schedule_id bigint,
    person_medication_id bigint,
    medication_take_id bigint,
    resolved_by_membership_id bigint,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    window_starts_on date NOT NULL,
    window_ends_on date,
    "position" integer NOT NULL,
    scheduled_at timestamp(6) without time zone,
    outcome character varying DEFAULT 'open'::character varying NOT NULL,
    reason character varying,
    note text,
    resolved_at timestamp(6) without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT chk_dose_occurrences_exact_source CHECK ((num_nonnulls(schedule_id, person_medication_id) = 1)),
    CONSTRAINT chk_dose_occurrences_note CHECK (((note IS NULL) OR (char_length(note) <= 2000))),
    CONSTRAINT chk_dose_occurrences_position CHECK (("position" > 0)),
    CONSTRAINT chk_dose_occurrences_reason CHECK (((reason IS NULL) OR ((reason)::text = ANY ((ARRAY['refused'::character varying, 'unwell'::character varying, 'asleep'::character varying, 'medicine_unavailable'::character varying, 'clinician_advice'::character varying, 'other'::character varying])::text[])))),
    CONSTRAINT chk_dose_occurrences_state CHECK (((((outcome)::text = 'open'::text) AND (medication_take_id IS NULL) AND (reason IS NULL) AND (note IS NULL) AND (resolved_at IS NULL) AND (resolved_by_membership_id IS NULL)) OR (((outcome)::text = 'not_taken'::text) AND (medication_take_id IS NULL) AND (resolved_at IS NOT NULL) AND (resolved_by_membership_id IS NOT NULL)) OR (((outcome)::text = 'taken'::text) AND (medication_take_id IS NOT NULL) AND (reason IS NULL) AND (note IS NULL) AND (resolved_at IS NOT NULL) AND (resolved_by_membership_id IS NOT NULL)))),
    CONSTRAINT chk_dose_occurrences_window_order CHECK ((window_ends_on >= window_starts_on))
);

ALTER TABLE ONLY public.medication_dose_occurrences FORCE ROW LEVEL SECURITY;


ALTER TABLE public.medication_dose_occurrences OWNER TO med_tracker_owner;

--
-- Name: medication_dose_occurrences_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medication_dose_occurrences_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_dose_occurrences_id_seq OWNER TO med_tracker_owner;

--
-- Name: medication_dose_occurrences_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medication_dose_occurrences_id_seq OWNED BY public.medication_dose_occurrences.id;


--
-- Name: medication_pause_periods; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medication_pause_periods (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    ended_at timestamp(6) without time zone,
    household_id bigint NOT NULL,
    imported_actor_references jsonb DEFAULT '{}'::jsonb NOT NULL,
    imported_context boolean DEFAULT false NOT NULL,
    legacy_context boolean DEFAULT false NOT NULL,
    note text,
    person_medication_id bigint,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    reason character varying NOT NULL,
    recorded_by_membership_id bigint,
    resumed_by_membership_id bigint,
    schedule_id bigint,
    started_at timestamp(6) without time zone,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT chk_medication_pause_periods_exactly_one_source CHECK ((num_nonnulls(schedule_id, person_medication_id) = 1)),
    CONSTRAINT chk_medication_pause_periods_interval CHECK (((started_at IS NULL) OR (ended_at IS NULL) OR (ended_at >= started_at))),
    CONSTRAINT chk_medication_pause_periods_legacy_context CHECK (((legacy_context AND ((reason)::text = 'reason_not_recorded'::text)) OR ((NOT legacy_context) AND ((reason)::text <> 'reason_not_recorded'::text) AND (started_at IS NOT NULL) AND ((recorded_by_membership_id IS NOT NULL) OR imported_context)))),
    CONSTRAINT chk_medication_pause_periods_reason CHECK (((reason)::text = ANY (ARRAY[('out_of_supply'::character varying)::text, ('temporarily_not_needed'::character varying)::text, ('clinician_advice'::character varying)::text, ('side_effects'::character varying)::text, ('other'::character varying)::text, ('reason_not_recorded'::character varying)::text]))),
    CONSTRAINT chk_medication_pause_periods_resuming_actor CHECK ((((ended_at IS NULL) AND (resumed_by_membership_id IS NULL)) OR ((ended_at IS NOT NULL) AND ((resumed_by_membership_id IS NOT NULL) OR imported_context))))
);

ALTER TABLE ONLY public.medication_pause_periods FORCE ROW LEVEL SECURITY;


ALTER TABLE public.medication_pause_periods OWNER TO med_tracker_owner;

--
-- Name: medication_pause_periods_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medication_pause_periods_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_pause_periods_id_seq OWNER TO med_tracker_owner;

--
-- Name: medication_pause_periods_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medication_pause_periods_id_seq OWNED BY public.medication_pause_periods.id;


--
-- Name: medication_review_evidence_records; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medication_review_evidence_records (
    id bigint NOT NULL,
    active_ingredient character varying,
    candidate_terms character varying[] DEFAULT '{}'::character varying[] NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    evidence_text text NOT NULL,
    interacting_terms character varying[] DEFAULT '{}'::character varying[] NOT NULL,
    label_section character varying NOT NULL,
    match_confidence character varying DEFAULT 'unknown'::character varying NOT NULL,
    match_status character varying DEFAULT 'unreviewed'::character varying NOT NULL,
    pharmacologic_classes character varying[] DEFAULT '{}'::character varying[] CONSTRAINT medication_review_evidence_recor_pharmacologic_classes_not_null NOT NULL,
    product_name character varying NOT NULL,
    retrieved_on date NOT NULL,
    risk_level character varying DEFAULT 'unknown'::character varying NOT NULL,
    source_effective_on date,
    source_name character varying NOT NULL,
    source_record_id character varying NOT NULL,
    source_url character varying NOT NULL,
    source_version character varying,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.medication_review_evidence_records OWNER TO med_tracker_owner;

--
-- Name: medication_review_evidence_records_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medication_review_evidence_records_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_review_evidence_records_id_seq OWNER TO med_tracker_owner;

--
-- Name: medication_review_evidence_records_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medication_review_evidence_records_id_seq OWNED BY public.medication_review_evidence_records.id;


--
-- Name: medication_review_evidence_refresh_runs; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medication_review_evidence_refresh_runs (
    id bigint NOT NULL,
    change_summary jsonb DEFAULT '{}'::jsonb NOT NULL,
    completed_at timestamp(6) without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    created_count integer DEFAULT 0 NOT NULL,
    error_message text,
    label_count integer DEFAULT 0 NOT NULL,
    missing_count integer DEFAULT 0 NOT NULL,
    source_last_updated date,
    started_at timestamp(6) without time zone,
    status integer DEFAULT 0 NOT NULL,
    unchanged_count integer DEFAULT 0 CONSTRAINT medication_review_evidence_refresh_run_unchanged_count_not_null NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    updated_count integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.medication_review_evidence_refresh_runs OWNER TO med_tracker_owner;

--
-- Name: medication_review_evidence_refresh_runs_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medication_review_evidence_refresh_runs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_review_evidence_refresh_runs_id_seq OWNER TO med_tracker_owner;

--
-- Name: medication_review_evidence_refresh_runs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medication_review_evidence_refresh_runs_id_seq OWNED BY public.medication_review_evidence_refresh_runs.id;


--
-- Name: medication_review_prompts; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medication_review_prompts (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    evidence_record_id bigint NOT NULL,
    evidence_source_checked_on date NOT NULL,
    evidence_source_effective_on date NOT NULL,
    evidence_source_name character varying NOT NULL,
    evidence_source_url character varying NOT NULL,
    evidence_source_version character varying NOT NULL,
    evidence_text text NOT NULL,
    household_id bigint NOT NULL,
    interacting_medication_id bigint NOT NULL,
    interacting_medication_name character varying NOT NULL,
    match_confidence character varying NOT NULL,
    match_reason text NOT NULL,
    match_type character varying NOT NULL,
    matched_term character varying NOT NULL,
    person_id bigint NOT NULL,
    practitioner_name character varying,
    practitioner_role character varying,
    primary_medication_id bigint NOT NULL,
    primary_medication_name character varying NOT NULL,
    review_note text,
    reviewed_by_membership_id bigint,
    reviewed_on date,
    risk_level character varying NOT NULL,
    source_instruction character varying NOT NULL,
    status character varying DEFAULT 'needs_review'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.medication_review_prompts FORCE ROW LEVEL SECURITY;


ALTER TABLE public.medication_review_prompts OWNER TO med_tracker_owner;

--
-- Name: medication_review_prompts_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medication_review_prompts_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_review_prompts_id_seq OWNER TO med_tracker_owner;

--
-- Name: medication_review_prompts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medication_review_prompts_id_seq OWNED BY public.medication_review_prompts.id;


--
-- Name: medication_takes; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medication_takes (
    id bigint NOT NULL,
    client_uuid character varying,
    created_at timestamp(6) without time zone NOT NULL,
    dose_amount numeric(10,2),
    dose_unit character varying,
    household_id bigint NOT NULL,
    person_medication_id bigint,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    schedule_id bigint,
    taken_at timestamp(6) without time zone,
    taken_from_location_id bigint,
    taken_from_medication_id bigint,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT chk_medication_takes_exactly_one_source CHECK ((num_nonnulls(schedule_id, person_medication_id) = 1))
);

ALTER TABLE ONLY public.medication_takes FORCE ROW LEVEL SECURITY;


ALTER TABLE public.medication_takes OWNER TO med_tracker_owner;

--
-- Name: medication_takes_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medication_takes_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_takes_id_seq OWNER TO med_tracker_owner;

--
-- Name: medication_takes_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medication_takes_id_seq OWNED BY public.medication_takes.id;


--
-- Name: medications; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.medications (
    id bigint NOT NULL,
    barcode character varying,
    category character varying,
    created_at timestamp(6) without time zone NOT NULL,
    created_by_membership_id bigint,
    current_supply numeric(10,2),
    default_schedule_config jsonb DEFAULT '{}'::jsonb NOT NULL,
    default_schedule_type integer DEFAULT 1 NOT NULL,
    description text,
    dmd_code character varying,
    dmd_concept_class character varying,
    dmd_system character varying,
    dose_amount double precision,
    dose_unit character varying,
    expiry_date date,
    friendly_name character varying,
    household_id bigint NOT NULL,
    location_id bigint NOT NULL,
    name character varying,
    expected_arrival_on date,
    ordered_at timestamp(6) without time zone,
    order_quantity numeric(10,2),
    order_supplier character varying,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    reorder_status integer,
    reorder_threshold numeric(10,2) DEFAULT 10.0 NOT NULL,
    reordered_at timestamp(6) without time zone,
    supply_at_last_restock numeric(10,2),
    updated_at timestamp(6) without time zone NOT NULL,
    warnings text
);

ALTER TABLE ONLY public.medications FORCE ROW LEVEL SECURITY;


ALTER TABLE public.medications OWNER TO med_tracker_owner;

--
-- Name: medications_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.medications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medications_id_seq OWNER TO med_tracker_owner;

--
-- Name: medications_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.medications_id_seq OWNED BY public.medications.id;


--
-- Name: native_device_tokens; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.native_device_tokens (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    apns_environment character varying,
    created_at timestamp(6) without time zone NOT NULL,
    device_token character varying NOT NULL,
    platform character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    user_agent character varying
);


ALTER TABLE public.native_device_tokens OWNER TO med_tracker_owner;

--
-- Name: native_device_tokens_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.native_device_tokens_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.native_device_tokens_id_seq OWNER TO med_tracker_owner;

--
-- Name: native_device_tokens_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.native_device_tokens_id_seq OWNED BY public.native_device_tokens.id;


--
-- Name: nhs_dmd_amp_trade_families; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_amp_trade_families (
    id bigint NOT NULL,
    amp_code character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    trade_family_id bigint NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.nhs_dmd_amp_trade_families OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_amp_trade_families_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_amp_trade_families_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_amp_trade_families_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_amp_trade_families_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_amp_trade_families_id_seq OWNED BY public.nhs_dmd_amp_trade_families.id;


--
-- Name: nhs_dmd_ampp_relationships; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_ampp_relationships (
    id bigint NOT NULL,
    amp_code character varying NOT NULL,
    ampp_code character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.nhs_dmd_ampp_relationships OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_ampp_relationships_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_ampp_relationships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_ampp_relationships_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_ampp_relationships_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_ampp_relationships_id_seq OWNED BY public.nhs_dmd_ampp_relationships.id;


--
-- Name: nhs_dmd_barcodes; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_barcodes (
    id bigint NOT NULL,
    amp_code character varying,
    code character varying NOT NULL,
    concept_class character varying,
    created_at timestamp(6) without time zone NOT NULL,
    display character varying NOT NULL,
    gtin character varying NOT NULL,
    system character varying DEFAULT 'https://dmd.nhs.uk'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    vmp_name character varying
);


ALTER TABLE public.nhs_dmd_barcodes OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_barcodes_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_barcodes_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_barcodes_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_barcodes_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_barcodes_id_seq OWNED BY public.nhs_dmd_barcodes.id;


--
-- Name: nhs_dmd_imports; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_imports (
    id bigint NOT NULL,
    archive_byte_size bigint,
    archive_checksum character varying,
    archive_key character varying,
    archive_service_name character varying,
    completed_at timestamp(6) without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    created_count integer DEFAULT 0 NOT NULL,
    error_message text,
    imported_count integer DEFAULT 0 NOT NULL,
    log text,
    processed_records integer DEFAULT 0 NOT NULL,
    skipped_count integer DEFAULT 0 NOT NULL,
    skipped_expired_count integer DEFAULT 0 NOT NULL,
    skipped_invalid_count integer DEFAULT 0 NOT NULL,
    skipped_missing_name_count integer DEFAULT 0 NOT NULL,
    started_at timestamp(6) without time zone,
    status integer DEFAULT 0 NOT NULL,
    total_records integer DEFAULT 0 NOT NULL,
    unchanged_count integer DEFAULT 0 NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    updated_count integer DEFAULT 0 NOT NULL,
    uploaded_filename character varying NOT NULL
);


ALTER TABLE public.nhs_dmd_imports OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_imports_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_imports_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_imports_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_imports_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_imports_id_seq OWNED BY public.nhs_dmd_imports.id;


--
-- Name: nhs_dmd_supplementary_releases; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_supplementary_releases (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    released_on date NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.nhs_dmd_supplementary_releases OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_supplementary_releases_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_supplementary_releases_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_supplementary_releases_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_supplementary_releases_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_supplementary_releases_id_seq OWNED BY public.nhs_dmd_supplementary_releases.id;


--
-- Name: nhs_dmd_trade_families; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_trade_families (
    id bigint NOT NULL,
    code character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    name character varying NOT NULL,
    trade_family_group_id bigint,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.nhs_dmd_trade_families OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_trade_families_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_trade_families_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_trade_families_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_trade_families_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_trade_families_id_seq OWNED BY public.nhs_dmd_trade_families.id;


--
-- Name: nhs_dmd_trade_family_groups; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.nhs_dmd_trade_family_groups (
    id bigint NOT NULL,
    code character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    name character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.nhs_dmd_trade_family_groups OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_trade_family_groups_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.nhs_dmd_trade_family_groups_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_trade_family_groups_id_seq OWNER TO med_tracker_owner;

--
-- Name: nhs_dmd_trade_family_groups_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.nhs_dmd_trade_family_groups_id_seq OWNED BY public.nhs_dmd_trade_family_groups.id;


--
-- Name: notification_events; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.notification_events (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    event_key character varying NOT NULL,
    event_type character varying NOT NULL,
    household_id bigint NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    person_id bigint,
    sent_at timestamp(6) without time zone,
    skipped_reason character varying,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.notification_events FORCE ROW LEVEL SECURITY;


ALTER TABLE public.notification_events OWNER TO med_tracker_owner;

--
-- Name: notification_events_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.notification_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.notification_events_id_seq OWNER TO med_tracker_owner;

--
-- Name: notification_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.notification_events_id_seq OWNED BY public.notification_events.id;


--
-- Name: notification_preferences; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.notification_preferences (
    id bigint NOT NULL,
    afternoon_time time without time zone DEFAULT '14:00:00'::time without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    dose_due_enabled boolean DEFAULT true NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    evening_time time without time zone DEFAULT '18:00:00'::time without time zone,
    household_id bigint NOT NULL,
    low_stock_enabled boolean DEFAULT true NOT NULL,
    missed_dose_enabled boolean DEFAULT true NOT NULL,
    morning_time time without time zone DEFAULT '08:00:00'::time without time zone,
    night_time time without time zone DEFAULT '22:00:00'::time without time zone,
    person_id bigint NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    private_text_enabled boolean DEFAULT false NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.notification_preferences FORCE ROW LEVEL SECURITY;


ALTER TABLE public.notification_preferences OWNER TO med_tracker_owner;

--
-- Name: notification_preferences_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.notification_preferences_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.notification_preferences_id_seq OWNER TO med_tracker_owner;

--
-- Name: notification_preferences_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.notification_preferences_id_seq OWNED BY public.notification_preferences.id;


--
-- Name: oauth_applications; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.oauth_applications (
    id bigint NOT NULL,
    account_id bigint,
    client_id character varying NOT NULL,
    client_kind character varying DEFAULT 'integration'::character varying NOT NULL,
    client_secret character varying,
    client_secret_hash character varying,
    created_at timestamp(6) without time zone NOT NULL,
    name character varying NOT NULL,
    redirect_uri character varying NOT NULL,
    scopes character varying NOT NULL,
    token_endpoint_auth_method character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT chk_oauth_applications_token_auth_method CHECK (((((token_endpoint_auth_method)::text = 'none'::text) AND (NULLIF((client_secret)::text, ''::text) IS NULL) AND (NULLIF((client_secret_hash)::text, ''::text) IS NULL)) OR (((token_endpoint_auth_method)::text = ANY ((ARRAY['client_secret_basic'::character varying, 'client_secret_post'::character varying, 'client_secret_basic client_secret_post'::character varying])::text[])) AND ((NULLIF((client_secret)::text, ''::text) IS NOT NULL) OR (NULLIF((client_secret_hash)::text, ''::text) IS NOT NULL))))),
    CONSTRAINT oauth_client_kind CHECK (((client_kind)::text = ANY (ARRAY[('integration'::character varying)::text, ('mobile'::character varying)::text])))
);


ALTER TABLE public.oauth_applications OWNER TO med_tracker_owner;

--
-- Name: oauth_applications_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.oauth_applications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.oauth_applications_id_seq OWNER TO med_tracker_owner;

--
-- Name: oauth_applications_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.oauth_applications_id_seq OWNED BY public.oauth_applications.id;


--
-- Name: oauth_grants; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.oauth_grants (
    id bigint NOT NULL,
    access_type character varying DEFAULT 'offline'::character varying NOT NULL,
    account_id bigint NOT NULL,
    authenticated_at timestamp(6) without time zone,
    client_kind character varying DEFAULT 'integration'::character varying NOT NULL,
    code character varying,
    code_challenge character varying,
    code_challenge_method character varying,
    created_at timestamp(6) without time zone NOT NULL,
    device_name character varying,
    expires_in timestamp(6) without time zone NOT NULL,
    household_membership_id bigint,
    last_used_at timestamp(6) without time zone,
    oauth_application_id bigint NOT NULL,
    permissions_version integer,
    person_id bigint,
    redirect_uri character varying,
    refresh_token character varying,
    refresh_token_hash character varying,
    revoked_at timestamp(6) without time zone,
    scopes character varying NOT NULL,
    token character varying,
    token_hash character varying,
    type character varying,
    updated_at timestamp(6) without time zone NOT NULL,
    CONSTRAINT oauth_grant_authority_boundary CHECK (((((client_kind)::text = 'integration'::text) AND (household_membership_id IS NOT NULL) AND (person_id IS NOT NULL) AND (permissions_version IS NOT NULL)) OR (((client_kind)::text = 'mobile'::text) AND (household_membership_id IS NULL) AND (person_id IS NULL) AND (permissions_version IS NULL) AND (authenticated_at IS NOT NULL) AND (last_used_at IS NOT NULL))))
);


ALTER TABLE public.oauth_grants OWNER TO med_tracker_owner;

--
-- Name: oauth_grants_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.oauth_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.oauth_grants_id_seq OWNER TO med_tracker_owner;

--
-- Name: oauth_grants_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.oauth_grants_id_seq OWNED BY public.oauth_grants.id;


--
-- Name: people; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.people (
    id bigint NOT NULL,
    account_id bigint,
    created_at timestamp(6) without time zone NOT NULL,
    date_of_birth date,
    email character varying,
    has_capacity boolean DEFAULT true NOT NULL,
    household_id bigint NOT NULL,
    name character varying NOT NULL,
    person_type integer DEFAULT 0 NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    professional_title character varying,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.people FORCE ROW LEVEL SECURITY;


ALTER TABLE public.people OWNER TO med_tracker_owner;

--
-- Name: people_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.people_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.people_id_seq OWNER TO med_tracker_owner;

--
-- Name: people_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.people_id_seq OWNED BY public.people.id;


--
-- Name: person_access_grants; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.person_access_grants (
    id bigint NOT NULL,
    access_level character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    expires_at timestamp(6) without time zone,
    granted_by_membership_id bigint,
    household_id bigint NOT NULL,
    household_membership_id bigint NOT NULL,
    missed_dose_notifications_enabled boolean DEFAULT false NOT NULL,
    person_id bigint NOT NULL,
    relationship_type character varying NOT NULL,
    revoked_at timestamp(6) without time zone,
    updated_at timestamp(6) without time zone NOT NULL,
    carer_relationship_id bigint
);

ALTER TABLE ONLY public.person_access_grants FORCE ROW LEVEL SECURITY;


ALTER TABLE public.person_access_grants OWNER TO med_tracker_owner;

--
-- Name: person_access_grants_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.person_access_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.person_access_grants_id_seq OWNER TO med_tracker_owner;

--
-- Name: person_access_grants_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.person_access_grants_id_seq OWNED BY public.person_access_grants.id;


--
-- Name: person_medications; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.person_medications (
    id bigint NOT NULL,
    active boolean DEFAULT true NOT NULL,
    administration_kind integer DEFAULT 1 NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    dose_amount numeric(10,2),
    dose_cycle integer,
    dose_unit character varying,
    household_id bigint NOT NULL,
    max_daily_doses integer,
    medication_id bigint NOT NULL,
    min_hours_between_doses integer,
    notes text,
    person_id bigint NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    "position" integer NOT NULL,
    retired_at timestamp(6) without time zone,
    source_dosage_option_id bigint,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.person_medications FORCE ROW LEVEL SECURITY;


ALTER TABLE public.person_medications OWNER TO med_tracker_owner;

--
-- Name: person_medications_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.person_medications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.person_medications_id_seq OWNER TO med_tracker_owner;

--
-- Name: person_medications_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.person_medications_id_seq OWNED BY public.person_medications.id;


--
-- Name: platform_admins; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.platform_admins (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    status character varying DEFAULT 'active'::character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.platform_admins OWNER TO med_tracker_owner;

--
-- Name: platform_admins_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.platform_admins_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.platform_admins_id_seq OWNER TO med_tracker_owner;

--
-- Name: platform_admins_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.platform_admins_id_seq OWNED BY public.platform_admins.id;


--
-- Name: push_subscriptions; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.push_subscriptions (
    id bigint NOT NULL,
    account_id bigint NOT NULL,
    auth character varying NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    endpoint character varying NOT NULL,
    p256dh character varying NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    user_agent character varying
);


ALTER TABLE public.push_subscriptions OWNER TO med_tracker_owner;

--
-- Name: push_subscriptions_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.push_subscriptions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.push_subscriptions_id_seq OWNER TO med_tracker_owner;

--
-- Name: push_subscriptions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.push_subscriptions_id_seq OWNED BY public.push_subscriptions.id;


--
-- Name: schedules; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.schedules (
    id bigint NOT NULL,
    active boolean DEFAULT true NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    dose_amount numeric(10,2),
    dose_cycle integer,
    dose_unit character varying,
    end_date date,
    frequency character varying,
    household_id bigint NOT NULL,
    max_daily_doses integer DEFAULT 4,
    medication_id bigint NOT NULL,
    min_hours_between_doses integer,
    notes text,
    person_id bigint NOT NULL,
    portable_id character varying DEFAULT (gen_random_uuid())::text NOT NULL,
    retired_at timestamp(6) without time zone,
    schedule_config jsonb DEFAULT '{}'::jsonb NOT NULL,
    schedule_type integer DEFAULT 0 NOT NULL,
    source_dosage_option_id bigint,
    start_date date,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.schedules FORCE ROW LEVEL SECURITY;


ALTER TABLE public.schedules OWNER TO med_tracker_owner;

--
-- Name: schedules_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.schedules_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.schedules_id_seq OWNER TO med_tracker_owner;

--
-- Name: schedules_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.schedules_id_seq OWNED BY public.schedules.id;


--
-- Name: schema_migrations; Type: TABLE; Schema: public; Owner: medtracker
--

CREATE TABLE public.schema_migrations (
    version character varying NOT NULL
);


ALTER TABLE public.schema_migrations OWNER TO medtracker;

--
-- Name: security_audit_events; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.security_audit_events (
    id bigint NOT NULL,
    actor_account_id bigint,
    actor_membership_id bigint,
    audit_context jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    event_type character varying NOT NULL,
    household_id bigint NOT NULL,
    ip character varying,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    request_id character varying,
    updated_at timestamp(6) without time zone NOT NULL
);

ALTER TABLE ONLY public.security_audit_events FORCE ROW LEVEL SECURITY;


ALTER TABLE public.security_audit_events OWNER TO med_tracker_owner;

--
-- Name: security_audit_events_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.security_audit_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.security_audit_events_id_seq OWNER TO med_tracker_owner;

--
-- Name: security_audit_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.security_audit_events_id_seq OWNED BY public.security_audit_events.id;


--
-- Name: storage_migration_runs; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.storage_migration_runs (
    id bigint NOT NULL,
    acceptance_verified_at timestamp(6) without time zone,
    created_at timestamp(6) without time zone NOT NULL,
    destination_service_name character varying NOT NULL,
    failed_count bigint DEFAULT 0 NOT NULL,
    final_reconciled_at timestamp(6) without time zone,
    finalized_at timestamp(6) without time zone,
    mirror_queue_drained_at timestamp(6) without time zone,
    phase character varying DEFAULT 'backfill'::character varying NOT NULL,
    processed_count bigint DEFAULT 0 NOT NULL,
    reconciled_at timestamp(6) without time zone,
    recovery_verified_at timestamp(6) without time zone,
    rollback_deadline timestamp(6) without time zone,
    run_id uuid DEFAULT gen_random_uuid() NOT NULL,
    source_service_name character varying NOT NULL,
    stable_blob_count bigint,
    updated_at timestamp(6) without time zone NOT NULL,
    verified_count bigint DEFAULT 0 NOT NULL
);


ALTER TABLE public.storage_migration_runs OWNER TO med_tracker_owner;

--
-- Name: storage_migration_runs_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.storage_migration_runs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.storage_migration_runs_id_seq OWNER TO med_tracker_owner;

--
-- Name: storage_migration_runs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.storage_migration_runs_id_seq OWNED BY public.storage_migration_runs.id;


--
-- Name: support_access_sessions; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.support_access_sessions (
    id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    ended_at timestamp(6) without time zone,
    expired_at timestamp(6) without time zone,
    expires_at timestamp(6) without time zone NOT NULL,
    household_id bigint NOT NULL,
    ip character varying,
    mfa_verified_at timestamp(6) without time zone,
    platform_admin_id bigint NOT NULL,
    reason text NOT NULL,
    request_id character varying,
    starts_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.support_access_sessions OWNER TO med_tracker_owner;

--
-- Name: support_access_sessions_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.support_access_sessions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.support_access_sessions_id_seq OWNER TO med_tracker_owner;

--
-- Name: support_access_sessions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.support_access_sessions_id_seq OWNED BY public.support_access_sessions.id;


--
-- Name: users; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.users (
    id bigint NOT NULL,
    active boolean DEFAULT true NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    email_address character varying NOT NULL,
    password_digest character varying,
    person_id bigint NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);


ALTER TABLE public.users OWNER TO med_tracker_owner;

--
-- Name: users_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.users_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.users_id_seq OWNER TO med_tracker_owner;

--
-- Name: users_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.users_id_seq OWNED BY public.users.id;


--
-- Name: versions; Type: TABLE; Schema: public; Owner: med_tracker_owner
--

CREATE TABLE public.versions (
    id bigint NOT NULL,
    actor_membership_id bigint,
    audit_context jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp(6) without time zone,
    event character varying NOT NULL,
    household_id bigint,
    ip character varying,
    item_id bigint NOT NULL,
    item_type character varying NOT NULL,
    object text,
    object_changes text,
    request_id character varying,
    whodunnit character varying
);


ALTER TABLE public.versions OWNER TO med_tracker_owner;

--
-- Name: versions_id_seq; Type: SEQUENCE; Schema: public; Owner: med_tracker_owner
--

CREATE SEQUENCE public.versions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.versions_id_seq OWNER TO med_tracker_owner;

--
-- Name: versions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: med_tracker_owner
--

ALTER SEQUENCE public.versions_id_seq OWNED BY public.versions.id;


--
-- Name: account_identities id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_identities ALTER COLUMN id SET DEFAULT nextval('public.account_identities_id_seq'::regclass);


--
-- Name: account_lockouts account_id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_lockouts ALTER COLUMN account_id SET DEFAULT nextval('public.account_lockouts_account_id_seq'::regclass);


--
-- Name: account_login_failures account_id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_login_failures ALTER COLUMN account_id SET DEFAULT nextval('public.account_login_failures_account_id_seq'::regclass);


--
-- Name: account_otp_keys id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_otp_keys ALTER COLUMN id SET DEFAULT nextval('public.account_otp_keys_id_seq'::regclass);


--
-- Name: account_webauthn_auth_challenges id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_auth_challenges ALTER COLUMN id SET DEFAULT nextval('public.account_webauthn_auth_challenges_id_seq'::regclass);


--
-- Name: account_webauthn_keys id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_keys ALTER COLUMN id SET DEFAULT nextval('public.account_webauthn_keys_id_seq'::regclass);


--
-- Name: account_webauthn_user_ids id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_user_ids ALTER COLUMN id SET DEFAULT nextval('public.account_webauthn_user_ids_id_seq'::regclass);


--
-- Name: accounts id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.accounts ALTER COLUMN id SET DEFAULT nextval('public.accounts_id_seq'::regclass);


--
-- Name: active_storage_attachments id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_attachments ALTER COLUMN id SET DEFAULT nextval('public.active_storage_attachments_id_seq'::regclass);


--
-- Name: active_storage_blobs id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_blobs ALTER COLUMN id SET DEFAULT nextval('public.active_storage_blobs_id_seq'::regclass);


--
-- Name: active_storage_variant_records id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_variant_records ALTER COLUMN id SET DEFAULT nextval('public.active_storage_variant_records_id_seq'::regclass);


--
-- Name: api_app_tokens id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_app_tokens ALTER COLUMN id SET DEFAULT nextval('public.api_app_tokens_id_seq'::regclass);


--
-- Name: api_change_events id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_change_events ALTER COLUMN id SET DEFAULT nextval('public.api_change_events_id_seq'::regclass);


--
-- Name: api_household_selection_grants id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_household_selection_grants ALTER COLUMN id SET DEFAULT nextval('public.api_household_selection_grants_id_seq'::regclass);


--
-- Name: api_idempotency_keys id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_idempotency_keys ALTER COLUMN id SET DEFAULT nextval('public.api_idempotency_keys_id_seq'::regclass);


--
-- Name: api_sessions id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_sessions ALTER COLUMN id SET DEFAULT nextval('public.api_sessions_id_seq'::regclass);


--
-- Name: api_tombstones id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_tombstones ALTER COLUMN id SET DEFAULT nextval('public.api_tombstones_id_seq'::regclass);


--
-- Name: app_settings id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.app_settings ALTER COLUMN id SET DEFAULT nextval('public.app_settings_id_seq'::regclass);


--
-- Name: audit_chain_heads id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_chain_heads ALTER COLUMN id SET DEFAULT nextval('public.audit_chain_heads_id_seq'::regclass);


--
-- Name: audit_checkpoints id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_checkpoints ALTER COLUMN id SET DEFAULT nextval('public.audit_checkpoints_id_seq'::regclass);


--
-- Name: audit_export_deliveries id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_export_deliveries ALTER COLUMN id SET DEFAULT nextval('public.audit_export_deliveries_id_seq'::regclass);


--
-- Name: audit_ledger_entries id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_ledger_entries ALTER COLUMN id SET DEFAULT nextval('public.audit_ledger_entries_id_seq'::regclass);


--
-- Name: audit_signing_keys id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_signing_keys ALTER COLUMN id SET DEFAULT nextval('public.audit_signing_keys_id_seq'::regclass);


--
-- Name: barcode_catalog_entries id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.barcode_catalog_entries ALTER COLUMN id SET DEFAULT nextval('public.barcode_catalog_entries_id_seq'::regclass);


--
-- Name: carer_relationships id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships ALTER COLUMN id SET DEFAULT nextval('public.carer_relationships_id_seq'::regclass);


--
-- Name: dosages id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.dosages ALTER COLUMN id SET DEFAULT nextval('public.dosages_id_seq'::regclass);


--
-- Name: health_event_medications id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications ALTER COLUMN id SET DEFAULT nextval('public.health_event_medications_id_seq'::regclass);


--
-- Name: health_events id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_events ALTER COLUMN id SET DEFAULT nextval('public.health_events_id_seq'::regclass);


--
-- Name: household_exports id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_exports ALTER COLUMN id SET DEFAULT nextval('public.household_exports_id_seq'::regclass);


--
-- Name: household_invitation_grants id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants ALTER COLUMN id SET DEFAULT nextval('public.household_invitation_grants_id_seq'::regclass);


--
-- Name: household_invitations id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitations ALTER COLUMN id SET DEFAULT nextval('public.household_invitations_id_seq'::regclass);


--
-- Name: household_memberships id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_memberships ALTER COLUMN id SET DEFAULT nextval('public.household_memberships_id_seq'::regclass);


--
-- Name: household_purge_runs id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_purge_runs ALTER COLUMN id SET DEFAULT nextval('public.household_purge_runs_id_seq'::regclass);


--
-- Name: household_retention_holds id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_retention_holds ALTER COLUMN id SET DEFAULT nextval('public.household_retention_holds_id_seq'::regclass);


--
-- Name: households id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.households ALTER COLUMN id SET DEFAULT nextval('public.households_id_seq'::regclass);


--
-- Name: location_memberships id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships ALTER COLUMN id SET DEFAULT nextval('public.location_memberships_id_seq'::regclass);


--
-- Name: locations id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.locations ALTER COLUMN id SET DEFAULT nextval('public.locations_id_seq'::regclass);


--
-- Name: medication_dose_occurrences id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences ALTER COLUMN id SET DEFAULT nextval('public.medication_dose_occurrences_id_seq'::regclass);


--
-- Name: medication_pause_periods id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods ALTER COLUMN id SET DEFAULT nextval('public.medication_pause_periods_id_seq'::regclass);


--
-- Name: medication_review_evidence_records id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_evidence_records ALTER COLUMN id SET DEFAULT nextval('public.medication_review_evidence_records_id_seq'::regclass);


--
-- Name: medication_review_evidence_refresh_runs id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_evidence_refresh_runs ALTER COLUMN id SET DEFAULT nextval('public.medication_review_evidence_refresh_runs_id_seq'::regclass);


--
-- Name: medication_review_prompts id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts ALTER COLUMN id SET DEFAULT nextval('public.medication_review_prompts_id_seq'::regclass);


--
-- Name: medication_takes id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes ALTER COLUMN id SET DEFAULT nextval('public.medication_takes_id_seq'::regclass);


--
-- Name: medications id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications ALTER COLUMN id SET DEFAULT nextval('public.medications_id_seq'::regclass);


--
-- Name: native_device_tokens id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.native_device_tokens ALTER COLUMN id SET DEFAULT nextval('public.native_device_tokens_id_seq'::regclass);


--
-- Name: nhs_dmd_amp_trade_families id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_amp_trade_families ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_amp_trade_families_id_seq'::regclass);


--
-- Name: nhs_dmd_ampp_relationships id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_ampp_relationships ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_ampp_relationships_id_seq'::regclass);


--
-- Name: nhs_dmd_barcodes id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_barcodes ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_barcodes_id_seq'::regclass);


--
-- Name: nhs_dmd_imports id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_imports ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_imports_id_seq'::regclass);


--
-- Name: nhs_dmd_supplementary_releases id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_supplementary_releases ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_supplementary_releases_id_seq'::regclass);


--
-- Name: nhs_dmd_trade_families id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_trade_families ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_trade_families_id_seq'::regclass);


--
-- Name: nhs_dmd_trade_family_groups id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_trade_family_groups ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_trade_family_groups_id_seq'::regclass);


--
-- Name: notification_events id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_events ALTER COLUMN id SET DEFAULT nextval('public.notification_events_id_seq'::regclass);


--
-- Name: notification_preferences id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_preferences ALTER COLUMN id SET DEFAULT nextval('public.notification_preferences_id_seq'::regclass);


--
-- Name: oauth_applications id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_applications ALTER COLUMN id SET DEFAULT nextval('public.oauth_applications_id_seq'::regclass);


--
-- Name: oauth_grants id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants ALTER COLUMN id SET DEFAULT nextval('public.oauth_grants_id_seq'::regclass);


--
-- Name: people id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.people ALTER COLUMN id SET DEFAULT nextval('public.people_id_seq'::regclass);


--
-- Name: person_access_grants id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants ALTER COLUMN id SET DEFAULT nextval('public.person_access_grants_id_seq'::regclass);


--
-- Name: person_medications id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications ALTER COLUMN id SET DEFAULT nextval('public.person_medications_id_seq'::regclass);


--
-- Name: platform_admins id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.platform_admins ALTER COLUMN id SET DEFAULT nextval('public.platform_admins_id_seq'::regclass);


--
-- Name: push_subscriptions id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.push_subscriptions ALTER COLUMN id SET DEFAULT nextval('public.push_subscriptions_id_seq'::regclass);


--
-- Name: schedules id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules ALTER COLUMN id SET DEFAULT nextval('public.schedules_id_seq'::regclass);


--
-- Name: security_audit_events id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.security_audit_events ALTER COLUMN id SET DEFAULT nextval('public.security_audit_events_id_seq'::regclass);


--
-- Name: storage_migration_runs id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.storage_migration_runs ALTER COLUMN id SET DEFAULT nextval('public.storage_migration_runs_id_seq'::regclass);


--
-- Name: support_access_sessions id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.support_access_sessions ALTER COLUMN id SET DEFAULT nextval('public.support_access_sessions_id_seq'::regclass);


--
-- Name: users id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.users ALTER COLUMN id SET DEFAULT nextval('public.users_id_seq'::regclass);


--
-- Name: versions id; Type: DEFAULT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.versions ALTER COLUMN id SET DEFAULT nextval('public.versions_id_seq'::regclass);


--
-- Name: account_identities account_identities_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_identities
    ADD CONSTRAINT account_identities_pkey PRIMARY KEY (id);


--
-- Name: account_lockouts account_lockouts_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_lockouts
    ADD CONSTRAINT account_lockouts_pkey PRIMARY KEY (account_id);


--
-- Name: account_login_failures account_login_failures_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_login_failures
    ADD CONSTRAINT account_login_failures_pkey PRIMARY KEY (account_id);


--
-- Name: account_otp_keys account_otp_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_otp_keys
    ADD CONSTRAINT account_otp_keys_pkey PRIMARY KEY (id);


--
-- Name: account_recovery_codes account_recovery_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_recovery_codes
    ADD CONSTRAINT account_recovery_codes_pkey PRIMARY KEY (id, code);


--
-- Name: account_webauthn_auth_challenges account_webauthn_auth_challenges_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_auth_challenges
    ADD CONSTRAINT account_webauthn_auth_challenges_pkey PRIMARY KEY (id);


--
-- Name: account_webauthn_keys account_webauthn_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_keys
    ADD CONSTRAINT account_webauthn_keys_pkey PRIMARY KEY (id);


--
-- Name: account_webauthn_user_ids account_webauthn_user_ids_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_user_ids
    ADD CONSTRAINT account_webauthn_user_ids_pkey PRIMARY KEY (id);


--
-- Name: accounts accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.accounts
    ADD CONSTRAINT accounts_pkey PRIMARY KEY (id);


--
-- Name: active_storage_attachments active_storage_attachments_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_attachments
    ADD CONSTRAINT active_storage_attachments_pkey PRIMARY KEY (id);


--
-- Name: active_storage_blobs active_storage_blobs_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_blobs
    ADD CONSTRAINT active_storage_blobs_pkey PRIMARY KEY (id);


--
-- Name: active_storage_variant_records active_storage_variant_records_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_variant_records
    ADD CONSTRAINT active_storage_variant_records_pkey PRIMARY KEY (id);


--
-- Name: api_app_tokens api_app_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_app_tokens
    ADD CONSTRAINT api_app_tokens_pkey PRIMARY KEY (id);


--
-- Name: api_change_events api_change_events_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_change_events
    ADD CONSTRAINT api_change_events_pkey PRIMARY KEY (id);


--
-- Name: api_household_selection_grants api_household_selection_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_household_selection_grants
    ADD CONSTRAINT api_household_selection_grants_pkey PRIMARY KEY (id);


--
-- Name: api_idempotency_keys api_idempotency_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_idempotency_keys
    ADD CONSTRAINT api_idempotency_keys_pkey PRIMARY KEY (id);


--
-- Name: api_sessions api_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_sessions
    ADD CONSTRAINT api_sessions_pkey PRIMARY KEY (id);


--
-- Name: api_tombstones api_tombstones_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_tombstones
    ADD CONSTRAINT api_tombstones_pkey PRIMARY KEY (id);


--
-- Name: app_settings app_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.app_settings
    ADD CONSTRAINT app_settings_pkey PRIMARY KEY (id);


--
-- Name: ar_internal_metadata ar_internal_metadata_pkey; Type: CONSTRAINT; Schema: public; Owner: medtracker
--

ALTER TABLE ONLY public.ar_internal_metadata
    ADD CONSTRAINT ar_internal_metadata_pkey PRIMARY KEY (key);


--
-- Name: audit_chain_heads audit_chain_heads_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_chain_heads
    ADD CONSTRAINT audit_chain_heads_pkey PRIMARY KEY (id);


--
-- Name: audit_checkpoints audit_checkpoints_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_checkpoints
    ADD CONSTRAINT audit_checkpoints_pkey PRIMARY KEY (id);


--
-- Name: audit_export_deliveries audit_export_deliveries_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_export_deliveries
    ADD CONSTRAINT audit_export_deliveries_pkey PRIMARY KEY (id);


--
-- Name: audit_ledger_entries audit_ledger_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_ledger_entries
    ADD CONSTRAINT audit_ledger_entries_pkey PRIMARY KEY (id);


--
-- Name: audit_signing_keys audit_signing_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_signing_keys
    ADD CONSTRAINT audit_signing_keys_pkey PRIMARY KEY (id);


--
-- Name: barcode_catalog_entries barcode_catalog_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.barcode_catalog_entries
    ADD CONSTRAINT barcode_catalog_entries_pkey PRIMARY KEY (id);


--
-- Name: carer_relationships carer_relationships_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT carer_relationships_pkey PRIMARY KEY (id);


--
-- Name: dosages dosages_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.dosages
    ADD CONSTRAINT dosages_pkey PRIMARY KEY (id);


--
-- Name: health_event_medications health_event_medications_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications
    ADD CONSTRAINT health_event_medications_pkey PRIMARY KEY (id);


--
-- Name: health_events health_events_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_events
    ADD CONSTRAINT health_events_pkey PRIMARY KEY (id);


--
-- Name: household_exports household_exports_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_exports
    ADD CONSTRAINT household_exports_pkey PRIMARY KEY (id);


--
-- Name: household_invitation_grants household_invitation_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants
    ADD CONSTRAINT household_invitation_grants_pkey PRIMARY KEY (id);


--
-- Name: household_invitations household_invitations_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitations
    ADD CONSTRAINT household_invitations_pkey PRIMARY KEY (id);


--
-- Name: household_memberships household_memberships_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_memberships
    ADD CONSTRAINT household_memberships_pkey PRIMARY KEY (id);


--
-- Name: household_purge_runs household_purge_runs_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_purge_runs
    ADD CONSTRAINT household_purge_runs_pkey PRIMARY KEY (id);


--
-- Name: household_retention_holds household_retention_holds_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_retention_holds
    ADD CONSTRAINT household_retention_holds_pkey PRIMARY KEY (id);


--
-- Name: households households_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.households
    ADD CONSTRAINT households_pkey PRIMARY KEY (id);


--
-- Name: location_memberships location_memberships_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT location_memberships_pkey PRIMARY KEY (id);


--
-- Name: locations locations_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (id);


--
-- Name: medication_dose_occurrences medication_dose_occurrences_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT medication_dose_occurrences_pkey PRIMARY KEY (id);


--
-- Name: medication_pause_periods medication_pause_periods_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT medication_pause_periods_pkey PRIMARY KEY (id);


--
-- Name: medication_review_evidence_records medication_review_evidence_records_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_evidence_records
    ADD CONSTRAINT medication_review_evidence_records_pkey PRIMARY KEY (id);


--
-- Name: medication_review_evidence_refresh_runs medication_review_evidence_refresh_runs_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_evidence_refresh_runs
    ADD CONSTRAINT medication_review_evidence_refresh_runs_pkey PRIMARY KEY (id);


--
-- Name: medication_review_prompts medication_review_prompts_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT medication_review_prompts_pkey PRIMARY KEY (id);


--
-- Name: medication_takes medication_takes_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT medication_takes_pkey PRIMARY KEY (id);


--
-- Name: medications medications_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT medications_pkey PRIMARY KEY (id);


--
-- Name: native_device_tokens native_device_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.native_device_tokens
    ADD CONSTRAINT native_device_tokens_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_amp_trade_families nhs_dmd_amp_trade_families_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_amp_trade_families
    ADD CONSTRAINT nhs_dmd_amp_trade_families_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_ampp_relationships nhs_dmd_ampp_relationships_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_ampp_relationships
    ADD CONSTRAINT nhs_dmd_ampp_relationships_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_barcodes nhs_dmd_barcodes_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_barcodes
    ADD CONSTRAINT nhs_dmd_barcodes_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_imports nhs_dmd_imports_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_imports
    ADD CONSTRAINT nhs_dmd_imports_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_supplementary_releases nhs_dmd_supplementary_releases_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_supplementary_releases
    ADD CONSTRAINT nhs_dmd_supplementary_releases_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_trade_families nhs_dmd_trade_families_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_trade_families
    ADD CONSTRAINT nhs_dmd_trade_families_pkey PRIMARY KEY (id);


--
-- Name: nhs_dmd_trade_family_groups nhs_dmd_trade_family_groups_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_trade_family_groups
    ADD CONSTRAINT nhs_dmd_trade_family_groups_pkey PRIMARY KEY (id);


--
-- Name: notification_events notification_events_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_events
    ADD CONSTRAINT notification_events_pkey PRIMARY KEY (id);


--
-- Name: notification_preferences notification_preferences_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT notification_preferences_pkey PRIMARY KEY (id);


--
-- Name: oauth_applications oauth_applications_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_applications
    ADD CONSTRAINT oauth_applications_pkey PRIMARY KEY (id);


--
-- Name: oauth_grants oauth_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants
    ADD CONSTRAINT oauth_grants_pkey PRIMARY KEY (id);


--
-- Name: people people_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.people
    ADD CONSTRAINT people_pkey PRIMARY KEY (id);


--
-- Name: person_access_grants person_access_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT person_access_grants_pkey PRIMARY KEY (id);


--
-- Name: person_medications person_medications_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT person_medications_pkey PRIMARY KEY (id);


--
-- Name: platform_admins platform_admins_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.platform_admins
    ADD CONSTRAINT platform_admins_pkey PRIMARY KEY (id);


--
-- Name: push_subscriptions push_subscriptions_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.push_subscriptions
    ADD CONSTRAINT push_subscriptions_pkey PRIMARY KEY (id);


--
-- Name: schedules schedules_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT schedules_pkey PRIMARY KEY (id);


--
-- Name: schema_migrations schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: medtracker
--

ALTER TABLE ONLY public.schema_migrations
    ADD CONSTRAINT schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: security_audit_events security_audit_events_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.security_audit_events
    ADD CONSTRAINT security_audit_events_pkey PRIMARY KEY (id);


--
-- Name: storage_migration_runs storage_migration_runs_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.storage_migration_runs
    ADD CONSTRAINT storage_migration_runs_pkey PRIMARY KEY (id);


--
-- Name: support_access_sessions support_access_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.support_access_sessions
    ADD CONSTRAINT support_access_sessions_pkey PRIMARY KEY (id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: versions versions_pkey; Type: CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.versions
    ADD CONSTRAINT versions_pkey PRIMARY KEY (id);


--
-- Name: idx_audit_checkpoint_chain_sequence; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_audit_checkpoint_chain_sequence ON public.audit_checkpoints USING btree (chain_key, chain_epoch, sequence);


--
-- Name: idx_audit_ledger_chain_sequence; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_audit_ledger_chain_sequence ON public.audit_ledger_entries USING btree (chain_key, chain_epoch, sequence);


--
-- Name: idx_dose_occurrence_person_medication_id_window; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_dose_occurrence_person_medication_id_window ON public.medication_dose_occurrences USING btree (person_medication_id, window_starts_on, "position") WHERE (person_medication_id IS NOT NULL);


--
-- Name: idx_dose_occurrence_schedule_id_window; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_dose_occurrence_schedule_id_window ON public.medication_dose_occurrences USING btree (schedule_id, window_starts_on, "position") WHERE (schedule_id IS NOT NULL);


--
-- Name: idx_dose_occurrences_household_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_dose_occurrences_household_portable_id ON public.medication_dose_occurrences USING btree (household_id, portable_id);


--
-- Name: idx_med_pause_periods_open_person_medication; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_med_pause_periods_open_person_medication ON public.medication_pause_periods USING btree (person_medication_id) WHERE ((ended_at IS NULL) AND (person_medication_id IS NOT NULL));


--
-- Name: idx_med_pause_periods_open_schedule; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_med_pause_periods_open_schedule ON public.medication_pause_periods USING btree (schedule_id) WHERE ((ended_at IS NULL) AND (schedule_id IS NOT NULL));


--
-- Name: idx_medication_review_prompts_unique_pair; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_medication_review_prompts_unique_pair ON public.medication_review_prompts USING btree (household_id, person_id, primary_medication_id, interacting_medication_id, evidence_record_id);


--
-- Name: idx_on_household_membership_id_person_id_6ddc5a2882; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_on_household_membership_id_person_id_6ddc5a2882 ON public.person_access_grants USING btree (household_membership_id, person_id) WHERE (revoked_at IS NULL);


--
-- Name: idx_on_pharmacologic_classes_df53f96090; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX idx_on_pharmacologic_classes_df53f96090 ON public.medication_review_evidence_records USING gin (pharmacologic_classes);


--
-- Name: idx_on_platform_admin_id_ended_at_0c69293a2c; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX idx_on_platform_admin_id_ended_at_0c69293a2c ON public.support_access_sessions USING btree (platform_admin_id, ended_at);


--
-- Name: idx_one_active_retention_hold_per_household; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX idx_one_active_retention_hold_per_household ON public.household_retention_holds USING btree (household_id) WHERE ((status)::text = 'active'::text);


--
-- Name: idx_person_access_grants_on_delegation_household; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX idx_person_access_grants_on_delegation_household ON public.person_access_grants USING btree (carer_relationship_id, household_id);


--
-- Name: index_account_active_session_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_active_session_keys_on_account_id ON public.account_active_session_keys USING btree (account_id);


--
-- Name: index_account_active_session_keys_on_account_id_and_session_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_account_active_session_keys_on_account_id_and_session_id ON public.account_active_session_keys USING btree (account_id, session_id);


--
-- Name: index_account_identities_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_identities_on_account_id ON public.account_identities USING btree (account_id);


--
-- Name: index_account_identities_on_provider_and_uid; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_account_identities_on_provider_and_uid ON public.account_identities USING btree (provider, uid);


--
-- Name: index_account_lockouts_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_lockouts_on_account_id ON public.account_lockouts USING btree (account_id);


--
-- Name: index_account_login_change_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_login_change_keys_on_account_id ON public.account_login_change_keys USING btree (account_id);


--
-- Name: index_account_login_failures_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_login_failures_on_account_id ON public.account_login_failures USING btree (account_id);


--
-- Name: index_account_password_reset_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_password_reset_keys_on_account_id ON public.account_password_reset_keys USING btree (account_id);


--
-- Name: index_account_remember_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_remember_keys_on_account_id ON public.account_remember_keys USING btree (account_id);


--
-- Name: index_account_verification_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_verification_keys_on_account_id ON public.account_verification_keys USING btree (account_id);


--
-- Name: index_account_webauthn_auth_challenges_on_challenge_digest; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_account_webauthn_auth_challenges_on_challenge_digest ON public.account_webauthn_auth_challenges USING btree (challenge_digest);


--
-- Name: index_account_webauthn_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_webauthn_keys_on_account_id ON public.account_webauthn_keys USING btree (account_id);


--
-- Name: index_account_webauthn_keys_on_webauthn_id_and_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_account_webauthn_keys_on_webauthn_id_and_account_id ON public.account_webauthn_keys USING btree (webauthn_id, account_id);


--
-- Name: index_account_webauthn_user_ids_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_account_webauthn_user_ids_on_account_id ON public.account_webauthn_user_ids USING btree (account_id);


--
-- Name: index_account_webauthn_user_ids_on_webauthn_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_account_webauthn_user_ids_on_webauthn_id ON public.account_webauthn_user_ids USING btree (webauthn_id);


--
-- Name: index_accounts_on_email; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_accounts_on_email ON public.accounts USING btree (email) WHERE (status = ANY (ARRAY[1, 2]));


--
-- Name: index_accounts_on_preferences; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_accounts_on_preferences ON public.accounts USING gin (preferences);


--
-- Name: index_active_storage_attachments_on_blob_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_active_storage_attachments_on_blob_id ON public.active_storage_attachments USING btree (blob_id);


--
-- Name: index_active_storage_attachments_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_active_storage_attachments_on_household_id ON public.active_storage_attachments USING btree (household_id);


--
-- Name: index_active_storage_attachments_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_active_storage_attachments_on_id_and_household_id ON public.active_storage_attachments USING btree (id, household_id);


--
-- Name: index_active_storage_attachments_uniqueness; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_active_storage_attachments_uniqueness ON public.active_storage_attachments USING btree (record_type, record_id, name, blob_id);


--
-- Name: index_active_storage_blobs_on_key; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_active_storage_blobs_on_key ON public.active_storage_blobs USING btree (key);


--
-- Name: index_active_storage_variant_records_uniqueness; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_active_storage_variant_records_uniqueness ON public.active_storage_variant_records USING btree (blob_id, variation_digest);


--
-- Name: index_api_app_tokens_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_app_tokens_on_account_id ON public.api_app_tokens USING btree (account_id);


--
-- Name: index_api_app_tokens_on_expires_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_app_tokens_on_expires_at ON public.api_app_tokens USING btree (expires_at);


--
-- Name: index_api_app_tokens_on_household_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_app_tokens_on_household_membership_id ON public.api_app_tokens USING btree (household_membership_id);


--
-- Name: index_api_app_tokens_on_membership_and_revoked_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_app_tokens_on_membership_and_revoked_at ON public.api_app_tokens USING btree (household_membership_id, revoked_at);


--
-- Name: index_api_app_tokens_on_token_digest; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_api_app_tokens_on_token_digest ON public.api_app_tokens USING btree (token_digest);


--
-- Name: index_api_change_events_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_change_events_on_account_id ON public.api_change_events USING btree (account_id);


--
-- Name: index_api_change_events_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_change_events_on_household_id ON public.api_change_events USING btree (household_id);


--
-- Name: index_api_change_events_on_household_id_and_occurred_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_change_events_on_household_id_and_occurred_at ON public.api_change_events USING btree (household_id, occurred_at);


--
-- Name: index_api_change_events_on_household_id_and_record_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_change_events_on_household_id_and_record_portable_id ON public.api_change_events USING btree (household_id, record_portable_id);


--
-- Name: index_api_change_events_on_household_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_change_events_on_household_membership_id ON public.api_change_events USING btree (household_membership_id);


--
-- Name: index_api_change_events_on_record_type_and_record_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_change_events_on_record_type_and_record_id ON public.api_change_events USING btree (record_type, record_id);


--
-- Name: index_api_household_selection_grants_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_household_selection_grants_on_account_id ON public.api_household_selection_grants USING btree (account_id);


--
-- Name: index_api_household_selection_grants_on_expires_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_household_selection_grants_on_expires_at ON public.api_household_selection_grants USING btree (expires_at);


--
-- Name: index_api_household_selection_grants_on_token_digest; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_api_household_selection_grants_on_token_digest ON public.api_household_selection_grants USING btree (token_digest);


--
-- Name: index_api_household_selection_grants_on_used_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_household_selection_grants_on_used_at ON public.api_household_selection_grants USING btree (used_at);


--
-- Name: index_api_idempotency_keys_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_idempotency_keys_on_account_id ON public.api_idempotency_keys USING btree (account_id);


--
-- Name: index_api_idempotency_keys_on_api_app_token_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_idempotency_keys_on_api_app_token_id ON public.api_idempotency_keys USING btree (api_app_token_id);


--
-- Name: index_api_idempotency_keys_on_api_session_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_idempotency_keys_on_api_session_id ON public.api_idempotency_keys USING btree (api_session_id);


--
-- Name: index_api_idempotency_keys_on_expires_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_idempotency_keys_on_expires_at ON public.api_idempotency_keys USING btree (expires_at);


--
-- Name: index_api_idempotency_keys_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_idempotency_keys_on_household_id ON public.api_idempotency_keys USING btree (household_id);


--
-- Name: index_api_idempotency_keys_on_household_id_and_key; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_api_idempotency_keys_on_household_id_and_key ON public.api_idempotency_keys USING btree (household_id, key);


--
-- Name: index_api_sessions_on_access_token_digest; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_api_sessions_on_access_token_digest ON public.api_sessions USING btree (access_token_digest);


--
-- Name: index_api_sessions_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_sessions_on_account_id ON public.api_sessions USING btree (account_id);


--
-- Name: index_api_sessions_on_household_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_sessions_on_household_membership_id ON public.api_sessions USING btree (household_membership_id);


--
-- Name: index_api_sessions_on_membership_and_revoked_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_sessions_on_membership_and_revoked_at ON public.api_sessions USING btree (household_membership_id, revoked_at);


--
-- Name: index_api_sessions_on_mfa_verified_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_sessions_on_mfa_verified_at ON public.api_sessions USING btree (mfa_verified_at);


--
-- Name: index_api_sessions_on_refresh_token_digest; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_api_sessions_on_refresh_token_digest ON public.api_sessions USING btree (refresh_token_digest);


--
-- Name: index_api_sessions_on_revoked_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_sessions_on_revoked_at ON public.api_sessions USING btree (revoked_at);


--
-- Name: index_api_tombstones_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_tombstones_on_account_id ON public.api_tombstones USING btree (account_id);


--
-- Name: index_api_tombstones_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_tombstones_on_household_id ON public.api_tombstones USING btree (household_id);


--
-- Name: index_api_tombstones_on_household_id_and_deleted_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_tombstones_on_household_id_and_deleted_at ON public.api_tombstones USING btree (household_id, deleted_at);


--
-- Name: index_api_tombstones_on_household_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_tombstones_on_household_membership_id ON public.api_tombstones USING btree (household_membership_id);


--
-- Name: index_api_tombstones_on_household_record; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_api_tombstones_on_household_record ON public.api_tombstones USING btree (household_id, record_type, record_portable_id);


--
-- Name: index_audit_chain_heads_on_chain_key; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_audit_chain_heads_on_chain_key ON public.audit_chain_heads USING btree (chain_key);


--
-- Name: index_audit_chain_heads_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_chain_heads_on_household_id ON public.audit_chain_heads USING btree (household_id);


--
-- Name: index_audit_checkpoints_on_audit_signing_key_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_checkpoints_on_audit_signing_key_id ON public.audit_checkpoints USING btree (audit_signing_key_id);


--
-- Name: index_audit_checkpoints_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_checkpoints_on_household_id ON public.audit_checkpoints USING btree (household_id);


--
-- Name: index_audit_export_deliveries_on_audit_checkpoint_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_audit_export_deliveries_on_audit_checkpoint_id ON public.audit_export_deliveries USING btree (audit_checkpoint_id);


--
-- Name: index_audit_export_deliveries_on_audit_ledger_entry_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_audit_export_deliveries_on_audit_ledger_entry_id ON public.audit_export_deliveries USING btree (audit_ledger_entry_id);


--
-- Name: index_audit_export_deliveries_on_status_and_next_attempt_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_export_deliveries_on_status_and_next_attempt_at ON public.audit_export_deliveries USING btree (status, next_attempt_at);


--
-- Name: index_audit_ledger_entries_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_ledger_entries_on_household_id ON public.audit_ledger_entries USING btree (household_id);


--
-- Name: index_audit_ledger_entries_on_household_id_and_occurred_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_ledger_entries_on_household_id_and_occurred_at ON public.audit_ledger_entries USING btree (household_id, occurred_at);


--
-- Name: index_audit_ledger_entries_on_retain_until; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_audit_ledger_entries_on_retain_until ON public.audit_ledger_entries USING btree (retain_until);


--
-- Name: index_audit_ledger_entries_on_source_table_and_source_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_audit_ledger_entries_on_source_table_and_source_id ON public.audit_ledger_entries USING btree (source_table, source_id);


--
-- Name: index_audit_signing_keys_on_key_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_audit_signing_keys_on_key_id ON public.audit_signing_keys USING btree (key_id);


--
-- Name: index_barcode_catalog_entries_on_gtin; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_barcode_catalog_entries_on_gtin ON public.barcode_catalog_entries USING btree (gtin);


--
-- Name: index_barcode_catalog_entries_on_source_and_gtin; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_barcode_catalog_entries_on_source_and_gtin ON public.barcode_catalog_entries USING btree (source, gtin);


--
-- Name: index_carer_relationships_on_active; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_carer_relationships_on_active ON public.carer_relationships USING btree (active);


--
-- Name: index_carer_relationships_on_carer_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_carer_relationships_on_carer_id ON public.carer_relationships USING btree (carer_id);


--
-- Name: index_carer_relationships_on_household_carer_patient; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_carer_relationships_on_household_carer_patient ON public.carer_relationships USING btree (household_id, carer_id, patient_id);


--
-- Name: index_carer_relationships_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_carer_relationships_on_household_id ON public.carer_relationships USING btree (household_id);


--
-- Name: index_carer_relationships_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_carer_relationships_on_id_and_household_id ON public.carer_relationships USING btree (id, household_id);


--
-- Name: index_carer_relationships_on_patient_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_carer_relationships_on_patient_id ON public.carer_relationships USING btree (patient_id);


--
-- Name: index_dosages_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_dosages_on_household_id ON public.dosages USING btree (household_id);


--
-- Name: index_dosages_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_dosages_on_household_id_and_portable_id ON public.dosages USING btree (household_id, portable_id);


--
-- Name: index_dosages_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_dosages_on_id_and_household_id ON public.dosages USING btree (id, household_id);


--
-- Name: index_dosages_on_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_dosages_on_medication_id ON public.dosages USING btree (medication_id);


--
-- Name: index_dosages_one_adult_default; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_dosages_one_adult_default ON public.dosages USING btree (medication_id) WHERE (default_for_adults = true);


--
-- Name: index_dosages_one_child_default; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_dosages_one_child_default ON public.dosages USING btree (medication_id) WHERE (default_for_children = true);


--
-- Name: index_health_event_medications_on_health_event_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_event_medications_on_health_event_id ON public.health_event_medications USING btree (health_event_id);


--
-- Name: index_health_event_medications_on_health_event_id_and_med_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_health_event_medications_on_health_event_id_and_med_id ON public.health_event_medications USING btree (health_event_id, medication_id) WHERE (medication_id IS NOT NULL);


--
-- Name: index_health_event_medications_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_event_medications_on_household_id ON public.health_event_medications USING btree (household_id);


--
-- Name: index_health_event_medications_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_health_event_medications_on_id_and_household_id ON public.health_event_medications USING btree (id, household_id);


--
-- Name: index_health_event_medications_on_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_event_medications_on_medication_id ON public.health_event_medications USING btree (medication_id);


--
-- Name: index_health_events_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_events_on_household_id ON public.health_events USING btree (household_id);


--
-- Name: index_health_events_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_health_events_on_household_id_and_portable_id ON public.health_events USING btree (household_id, portable_id);


--
-- Name: index_health_events_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_health_events_on_id_and_household_id ON public.health_events USING btree (id, household_id);


--
-- Name: index_health_events_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_events_on_person_id ON public.health_events USING btree (person_id);


--
-- Name: index_health_events_on_person_id_and_event_kind_and_started_on; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_events_on_person_id_and_event_kind_and_started_on ON public.health_events USING btree (person_id, event_kind, started_on);


--
-- Name: index_health_events_on_person_id_and_started_on_and_ended_on; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_health_events_on_person_id_and_started_on_and_ended_on ON public.health_events USING btree (person_id, started_on, ended_on);


--
-- Name: index_household_exports_on_expires_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_exports_on_expires_at ON public.household_exports USING btree (expires_at);


--
-- Name: index_household_exports_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_exports_on_household_id ON public.household_exports USING btree (household_id);


--
-- Name: index_household_exports_on_household_id_and_status; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_exports_on_household_id_and_status ON public.household_exports USING btree (household_id, status);


--
-- Name: index_household_exports_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_exports_on_id_and_household_id ON public.household_exports USING btree (id, household_id);


--
-- Name: index_household_exports_on_requested_by_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_exports_on_requested_by_account_id ON public.household_exports USING btree (requested_by_account_id);


--
-- Name: index_household_invitation_grants_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_invitation_grants_on_household_id ON public.household_invitation_grants USING btree (household_id);


--
-- Name: index_household_invitation_grants_on_household_invitation_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_invitation_grants_on_household_invitation_id ON public.household_invitation_grants USING btree (household_invitation_id);


--
-- Name: index_household_invitation_grants_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_invitation_grants_on_id_and_household_id ON public.household_invitation_grants USING btree (id, household_id);


--
-- Name: index_household_invitation_grants_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_invitation_grants_on_person_id ON public.household_invitation_grants USING btree (person_id);


--
-- Name: index_household_invitations_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_invitations_on_household_id ON public.household_invitations USING btree (household_id);


--
-- Name: index_household_invitations_on_household_id_and_email; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_invitations_on_household_id_and_email ON public.household_invitations USING btree (household_id, email) WHERE ((accepted_at IS NULL) AND (revoked_at IS NULL));


--
-- Name: index_household_invitations_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_invitations_on_id_and_household_id ON public.household_invitations USING btree (id, household_id);


--
-- Name: index_household_invitations_on_invited_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_invitations_on_invited_by_membership_id ON public.household_invitations USING btree (invited_by_membership_id);


--
-- Name: index_household_invitations_on_token_digest; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_invitations_on_token_digest ON public.household_invitations USING btree (token_digest);


--
-- Name: index_household_memberships_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_memberships_on_account_id ON public.household_memberships USING btree (account_id);


--
-- Name: index_household_memberships_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_memberships_on_household_id ON public.household_memberships USING btree (household_id);


--
-- Name: index_household_memberships_on_household_id_and_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_memberships_on_household_id_and_account_id ON public.household_memberships USING btree (household_id, account_id);


--
-- Name: index_household_memberships_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_memberships_on_id_and_household_id ON public.household_memberships USING btree (id, household_id);


--
-- Name: index_household_memberships_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_memberships_on_person_id ON public.household_memberships USING btree (person_id) WHERE (person_id IS NOT NULL);


--
-- Name: index_household_purge_runs_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_purge_runs_on_household_id ON public.household_purge_runs USING btree (household_id);


--
-- Name: index_household_purge_runs_on_household_id_and_status; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_purge_runs_on_household_id_and_status ON public.household_purge_runs USING btree (household_id, status);


--
-- Name: index_household_purge_runs_on_requested_by_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_purge_runs_on_requested_by_account_id ON public.household_purge_runs USING btree (requested_by_account_id);


--
-- Name: index_household_retention_holds_on_approved_by_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_retention_holds_on_approved_by_account_id ON public.household_retention_holds USING btree (approved_by_account_id);


--
-- Name: index_household_retention_holds_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_retention_holds_on_household_id ON public.household_retention_holds USING btree (household_id);


--
-- Name: index_household_retention_holds_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_household_retention_holds_on_id_and_household_id ON public.household_retention_holds USING btree (id, household_id);


--
-- Name: index_household_retention_holds_on_released_by_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_retention_holds_on_released_by_account_id ON public.household_retention_holds USING btree (released_by_account_id);


--
-- Name: index_household_retention_holds_on_status_and_review_on; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_household_retention_holds_on_status_and_review_on ON public.household_retention_holds USING btree (status, review_on);


--
-- Name: index_households_on_created_by_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_households_on_created_by_account_id ON public.households USING btree (created_by_account_id);


--
-- Name: index_households_on_lifecycle_state; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_households_on_lifecycle_state ON public.households USING btree (lifecycle_state);


--
-- Name: index_households_on_slug; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_households_on_slug ON public.households USING btree (slug);


--
-- Name: index_location_memberships_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_location_memberships_on_household_id ON public.location_memberships USING btree (household_id);


--
-- Name: index_location_memberships_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_location_memberships_on_id_and_household_id ON public.location_memberships USING btree (id, household_id);


--
-- Name: index_location_memberships_on_location_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_location_memberships_on_location_id ON public.location_memberships USING btree (location_id);


--
-- Name: index_location_memberships_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_location_memberships_on_person_id ON public.location_memberships USING btree (person_id);


--
-- Name: index_location_memberships_on_person_id_and_location_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_location_memberships_on_person_id_and_location_id ON public.location_memberships USING btree (person_id, location_id);


--
-- Name: index_locations_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_locations_on_household_id ON public.locations USING btree (household_id);


--
-- Name: index_locations_on_household_id_and_lower_name; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_locations_on_household_id_and_lower_name ON public.locations USING btree (household_id, lower((name)::text)) WHERE (household_id IS NOT NULL);


--
-- Name: index_locations_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_locations_on_household_id_and_portable_id ON public.locations USING btree (household_id, portable_id);


--
-- Name: index_locations_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_locations_on_id_and_household_id ON public.locations USING btree (id, household_id);


--
-- Name: index_locations_on_name_trigram; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_locations_on_name_trigram ON public.locations USING gin (name public.gin_trgm_ops);


--
-- Name: index_medication_dose_occurrences_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_dose_occurrences_on_household_id ON public.medication_dose_occurrences USING btree (household_id);


--
-- Name: index_medication_dose_occurrences_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_dose_occurrences_on_id_and_household_id ON public.medication_dose_occurrences USING btree (id, household_id);


--
-- Name: index_medication_dose_occurrences_on_medication_take_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_dose_occurrences_on_medication_take_id ON public.medication_dose_occurrences USING btree (medication_take_id);


--
-- Name: index_medication_dose_occurrences_on_person_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_dose_occurrences_on_person_medication_id ON public.medication_dose_occurrences USING btree (person_medication_id);


--
-- Name: index_medication_dose_occurrences_on_resolved_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_dose_occurrences_on_resolved_by_membership_id ON public.medication_dose_occurrences USING btree (resolved_by_membership_id);


--
-- Name: index_medication_dose_occurrences_on_schedule_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_dose_occurrences_on_schedule_id ON public.medication_dose_occurrences USING btree (schedule_id);


--
-- Name: index_medication_pause_periods_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_pause_periods_on_household_id ON public.medication_pause_periods USING btree (household_id);


--
-- Name: index_medication_pause_periods_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_pause_periods_on_household_id_and_portable_id ON public.medication_pause_periods USING btree (household_id, portable_id);


--
-- Name: index_medication_pause_periods_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_pause_periods_on_id_and_household_id ON public.medication_pause_periods USING btree (id, household_id);


--
-- Name: index_medication_pause_periods_on_person_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_pause_periods_on_person_medication_id ON public.medication_pause_periods USING btree (person_medication_id);


--
-- Name: index_medication_pause_periods_on_recorded_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_pause_periods_on_recorded_by_membership_id ON public.medication_pause_periods USING btree (recorded_by_membership_id);


--
-- Name: index_medication_pause_periods_on_resumed_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_pause_periods_on_resumed_by_membership_id ON public.medication_pause_periods USING btree (resumed_by_membership_id);


--
-- Name: index_medication_pause_periods_on_schedule_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_pause_periods_on_schedule_id ON public.medication_pause_periods USING btree (schedule_id);


--
-- Name: index_medication_review_evidence_records_on_candidate_terms; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_evidence_records_on_candidate_terms ON public.medication_review_evidence_records USING gin (candidate_terms);


--
-- Name: index_medication_review_evidence_records_on_interacting_terms; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_evidence_records_on_interacting_terms ON public.medication_review_evidence_records USING gin (interacting_terms);


--
-- Name: index_medication_review_evidence_records_on_match_status; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_evidence_records_on_match_status ON public.medication_review_evidence_records USING btree (match_status);


--
-- Name: index_medication_review_evidence_records_on_source_record_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_review_evidence_records_on_source_record_id ON public.medication_review_evidence_records USING btree (source_record_id);


--
-- Name: index_medication_review_prompts_on_evidence_record_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_evidence_record_id ON public.medication_review_prompts USING btree (evidence_record_id);


--
-- Name: index_medication_review_prompts_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_household_id ON public.medication_review_prompts USING btree (household_id);


--
-- Name: index_medication_review_prompts_on_household_id_and_status; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_household_id_and_status ON public.medication_review_prompts USING btree (household_id, status);


--
-- Name: index_medication_review_prompts_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_review_prompts_on_id_and_household_id ON public.medication_review_prompts USING btree (id, household_id);


--
-- Name: index_medication_review_prompts_on_interacting_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_interacting_medication_id ON public.medication_review_prompts USING btree (interacting_medication_id);


--
-- Name: index_medication_review_prompts_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_person_id ON public.medication_review_prompts USING btree (person_id);


--
-- Name: index_medication_review_prompts_on_primary_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_primary_medication_id ON public.medication_review_prompts USING btree (primary_medication_id);


--
-- Name: index_medication_review_prompts_on_reviewed_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_review_prompts_on_reviewed_by_membership_id ON public.medication_review_prompts USING btree (reviewed_by_membership_id);


--
-- Name: index_medication_takes_on_client_uuid; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_takes_on_client_uuid ON public.medication_takes USING btree (client_uuid) WHERE (client_uuid IS NOT NULL);


--
-- Name: index_medication_takes_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_takes_on_household_id ON public.medication_takes USING btree (household_id);


--
-- Name: index_medication_takes_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_takes_on_household_id_and_portable_id ON public.medication_takes USING btree (household_id, portable_id);


--
-- Name: index_medication_takes_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medication_takes_on_id_and_household_id ON public.medication_takes USING btree (id, household_id);


--
-- Name: index_medication_takes_on_person_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_takes_on_person_medication_id ON public.medication_takes USING btree (person_medication_id);


--
-- Name: index_medication_takes_on_schedule_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_takes_on_schedule_id ON public.medication_takes USING btree (schedule_id);


--
-- Name: index_medication_takes_on_taken_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_takes_on_taken_at ON public.medication_takes USING btree (taken_at);


--
-- Name: index_medication_takes_on_taken_from_location_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_takes_on_taken_from_location_id ON public.medication_takes USING btree (taken_from_location_id);


--
-- Name: index_medication_takes_on_taken_from_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medication_takes_on_taken_from_medication_id ON public.medication_takes USING btree (taken_from_medication_id);


--
-- Name: index_medications_on_barcode; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medications_on_barcode ON public.medications USING btree (barcode) WHERE ((barcode IS NOT NULL) AND ((barcode)::text <> ''::text));


--
-- Name: index_medications_on_barcode_trigram; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_barcode_trigram ON public.medications USING gin (barcode public.gin_trgm_ops);


--
-- Name: index_medications_on_category_trigram; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_category_trigram ON public.medications USING gin (category public.gin_trgm_ops);


--
-- Name: index_medications_on_created_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_created_by_membership_id ON public.medications USING btree (created_by_membership_id);


--
-- Name: index_medications_on_default_schedule_type; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_default_schedule_type ON public.medications USING btree (default_schedule_type);


--
-- Name: index_medications_on_dmd_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_dmd_code ON public.medications USING btree (dmd_code);


--
-- Name: index_medications_on_dmd_code_trigram; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_dmd_code_trigram ON public.medications USING gin (dmd_code public.gin_trgm_ops);


--
-- Name: index_medications_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_household_id ON public.medications USING btree (household_id);


--
-- Name: index_medications_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medications_on_household_id_and_portable_id ON public.medications USING btree (household_id, portable_id);


--
-- Name: index_medications_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_medications_on_id_and_household_id ON public.medications USING btree (id, household_id);


--
-- Name: index_medications_on_location_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_location_id ON public.medications USING btree (location_id);


--
-- Name: index_medications_on_name_trigram; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_medications_on_name_trigram ON public.medications USING gin (name public.gin_trgm_ops);


--
-- Name: index_native_device_tokens_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_native_device_tokens_on_account_id ON public.native_device_tokens USING btree (account_id);


--
-- Name: index_native_device_tokens_on_account_id_and_platform; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_native_device_tokens_on_account_id_and_platform ON public.native_device_tokens USING btree (account_id, platform);


--
-- Name: index_native_device_tokens_on_device_token; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_native_device_tokens_on_device_token ON public.native_device_tokens USING btree (device_token);


--
-- Name: index_nhs_dmd_amp_trade_families_on_amp_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_amp_trade_families_on_amp_code ON public.nhs_dmd_amp_trade_families USING btree (amp_code);


--
-- Name: index_nhs_dmd_amp_trade_families_on_trade_family_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_nhs_dmd_amp_trade_families_on_trade_family_id ON public.nhs_dmd_amp_trade_families USING btree (trade_family_id);


--
-- Name: index_nhs_dmd_ampp_relationships_on_amp_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_nhs_dmd_ampp_relationships_on_amp_code ON public.nhs_dmd_ampp_relationships USING btree (amp_code);


--
-- Name: index_nhs_dmd_ampp_relationships_on_ampp_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_ampp_relationships_on_ampp_code ON public.nhs_dmd_ampp_relationships USING btree (ampp_code);


--
-- Name: index_nhs_dmd_barcodes_on_amp_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_nhs_dmd_barcodes_on_amp_code ON public.nhs_dmd_barcodes USING btree (amp_code);


--
-- Name: index_nhs_dmd_barcodes_on_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_nhs_dmd_barcodes_on_code ON public.nhs_dmd_barcodes USING btree (code);


--
-- Name: index_nhs_dmd_barcodes_on_gtin; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_barcodes_on_gtin ON public.nhs_dmd_barcodes USING btree (gtin);


--
-- Name: index_nhs_dmd_imports_one_active; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_imports_one_active ON public.nhs_dmd_imports USING btree ((1)) WHERE (status = ANY (ARRAY[0, 1, 2, 3]));


--
-- Name: index_nhs_dmd_supplementary_releases_on_released_on; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_supplementary_releases_on_released_on ON public.nhs_dmd_supplementary_releases USING btree (released_on);


--
-- Name: index_nhs_dmd_trade_families_on_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_trade_families_on_code ON public.nhs_dmd_trade_families USING btree (code);


--
-- Name: index_nhs_dmd_trade_families_on_trade_family_group_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_nhs_dmd_trade_families_on_trade_family_group_id ON public.nhs_dmd_trade_families USING btree (trade_family_group_id);


--
-- Name: index_nhs_dmd_trade_family_groups_on_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_nhs_dmd_trade_family_groups_on_code ON public.nhs_dmd_trade_family_groups USING btree (code);


--
-- Name: index_notification_events_on_event_type_and_event_key; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_notification_events_on_event_type_and_event_key ON public.notification_events USING btree (event_type, event_key);


--
-- Name: index_notification_events_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_notification_events_on_household_id ON public.notification_events USING btree (household_id);


--
-- Name: index_notification_events_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_notification_events_on_id_and_household_id ON public.notification_events USING btree (id, household_id);


--
-- Name: index_notification_events_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_notification_events_on_person_id ON public.notification_events USING btree (person_id);


--
-- Name: index_notification_events_on_sent_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_notification_events_on_sent_at ON public.notification_events USING btree (sent_at);


--
-- Name: index_notification_preferences_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_notification_preferences_on_household_id ON public.notification_preferences USING btree (household_id);


--
-- Name: index_notification_preferences_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_notification_preferences_on_household_id_and_portable_id ON public.notification_preferences USING btree (household_id, portable_id);


--
-- Name: index_notification_preferences_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_notification_preferences_on_id_and_household_id ON public.notification_preferences USING btree (id, household_id);


--
-- Name: index_notification_preferences_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_notification_preferences_on_person_id ON public.notification_preferences USING btree (person_id);


--
-- Name: index_oauth_applications_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_oauth_applications_on_account_id ON public.oauth_applications USING btree (account_id);


--
-- Name: index_oauth_applications_on_client_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_applications_on_client_id ON public.oauth_applications USING btree (client_id);


--
-- Name: index_oauth_applications_on_id_and_client_kind; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_applications_on_id_and_client_kind ON public.oauth_applications USING btree (id, client_kind);


--
-- Name: index_oauth_grants_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_oauth_grants_on_account_id ON public.oauth_grants USING btree (account_id);


--
-- Name: index_oauth_grants_on_household_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_oauth_grants_on_household_membership_id ON public.oauth_grants USING btree (household_membership_id);


--
-- Name: index_oauth_grants_on_oauth_application_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_oauth_grants_on_oauth_application_id ON public.oauth_grants USING btree (oauth_application_id);


--
-- Name: index_oauth_grants_on_oauth_application_id_and_code; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_grants_on_oauth_application_id_and_code ON public.oauth_grants USING btree (oauth_application_id, code);


--
-- Name: index_oauth_grants_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_oauth_grants_on_person_id ON public.oauth_grants USING btree (person_id);


--
-- Name: index_oauth_grants_on_refresh_token; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_grants_on_refresh_token ON public.oauth_grants USING btree (refresh_token);


--
-- Name: index_oauth_grants_on_refresh_token_hash; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_grants_on_refresh_token_hash ON public.oauth_grants USING btree (refresh_token_hash);


--
-- Name: index_oauth_grants_on_token; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_grants_on_token ON public.oauth_grants USING btree (token);


--
-- Name: index_oauth_grants_on_token_hash; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_oauth_grants_on_token_hash ON public.oauth_grants USING btree (token_hash);


--
-- Name: index_people_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_people_on_account_id ON public.people USING btree (account_id);


--
-- Name: index_people_on_email_present_unique; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_people_on_email_present_unique ON public.people USING btree (email) WHERE ((email IS NOT NULL) AND (btrim((email)::text) <> ''::text));


--
-- Name: index_people_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_people_on_household_id ON public.people USING btree (household_id);


--
-- Name: index_people_on_household_id_and_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_people_on_household_id_and_account_id ON public.people USING btree (household_id, account_id) WHERE ((household_id IS NOT NULL) AND (account_id IS NOT NULL));


--
-- Name: index_people_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_people_on_household_id_and_portable_id ON public.people USING btree (household_id, portable_id);


--
-- Name: index_people_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_people_on_id_and_household_id ON public.people USING btree (id, household_id);


--
-- Name: index_people_on_name_trigram; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_people_on_name_trigram ON public.people USING gin (name public.gin_trgm_ops);


--
-- Name: index_people_on_person_type; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_people_on_person_type ON public.people USING btree (person_type);


--
-- Name: index_person_access_grants_on_granted_by_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_access_grants_on_granted_by_membership_id ON public.person_access_grants USING btree (granted_by_membership_id);


--
-- Name: index_person_access_grants_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_access_grants_on_household_id ON public.person_access_grants USING btree (household_id);


--
-- Name: index_person_access_grants_on_household_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_access_grants_on_household_membership_id ON public.person_access_grants USING btree (household_membership_id);


--
-- Name: index_person_access_grants_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_person_access_grants_on_id_and_household_id ON public.person_access_grants USING btree (id, household_id);


--
-- Name: index_person_access_grants_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_access_grants_on_person_id ON public.person_access_grants USING btree (person_id);


--
-- Name: index_person_medications_on_active; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_active ON public.person_medications USING btree (active);


--
-- Name: index_person_medications_on_administration_kind; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_administration_kind ON public.person_medications USING btree (administration_kind);


--
-- Name: index_person_medications_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_household_id ON public.person_medications USING btree (household_id);


--
-- Name: index_person_medications_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_person_medications_on_household_id_and_portable_id ON public.person_medications USING btree (household_id, portable_id);


--
-- Name: index_person_medications_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_person_medications_on_id_and_household_id ON public.person_medications USING btree (id, household_id);


--
-- Name: index_person_medications_on_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_medication_id ON public.person_medications USING btree (medication_id);


--
-- Name: index_person_medications_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_person_id ON public.person_medications USING btree (person_id);


--
-- Name: index_person_medications_on_person_id_and_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_person_medications_on_person_id_and_medication_id ON public.person_medications USING btree (person_id, medication_id) WHERE (retired_at IS NULL);


--
-- Name: index_person_medications_on_person_id_and_position; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_person_id_and_position ON public.person_medications USING btree (person_id, "position");


--
-- Name: index_person_medications_on_retired_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_retired_at ON public.person_medications USING btree (retired_at);


--
-- Name: index_person_medications_on_source_dosage_option_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_person_medications_on_source_dosage_option_id ON public.person_medications USING btree (source_dosage_option_id);


--
-- Name: index_platform_admins_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_platform_admins_on_account_id ON public.platform_admins USING btree (account_id);


--
-- Name: index_push_subscriptions_on_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_push_subscriptions_on_account_id ON public.push_subscriptions USING btree (account_id);


--
-- Name: index_push_subscriptions_on_endpoint; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_push_subscriptions_on_endpoint ON public.push_subscriptions USING btree (endpoint);


--
-- Name: index_schedules_on_active; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_active ON public.schedules USING btree (active);


--
-- Name: index_schedules_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_household_id ON public.schedules USING btree (household_id);


--
-- Name: index_schedules_on_household_id_and_portable_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_schedules_on_household_id_and_portable_id ON public.schedules USING btree (household_id, portable_id);


--
-- Name: index_schedules_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_schedules_on_id_and_household_id ON public.schedules USING btree (id, household_id);


--
-- Name: index_schedules_on_medication_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_medication_id ON public.schedules USING btree (medication_id);


--
-- Name: index_schedules_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_person_id ON public.schedules USING btree (person_id);


--
-- Name: index_schedules_on_retired_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_retired_at ON public.schedules USING btree (retired_at);


--
-- Name: index_schedules_on_schedule_config; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_schedule_config ON public.schedules USING gin (schedule_config);


--
-- Name: index_schedules_on_schedule_type; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_schedule_type ON public.schedules USING btree (schedule_type);


--
-- Name: index_schedules_on_source_dosage_option_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_schedules_on_source_dosage_option_id ON public.schedules USING btree (source_dosage_option_id);


--
-- Name: index_security_audit_events_on_actor_account_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_security_audit_events_on_actor_account_id ON public.security_audit_events USING btree (actor_account_id);


--
-- Name: index_security_audit_events_on_actor_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_security_audit_events_on_actor_membership_id ON public.security_audit_events USING btree (actor_membership_id);


--
-- Name: index_security_audit_events_on_event_type; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_security_audit_events_on_event_type ON public.security_audit_events USING btree (event_type);


--
-- Name: index_security_audit_events_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_security_audit_events_on_household_id ON public.security_audit_events USING btree (household_id);


--
-- Name: index_security_audit_events_on_household_id_and_created_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_security_audit_events_on_household_id_and_created_at ON public.security_audit_events USING btree (household_id, created_at);


--
-- Name: index_security_audit_events_on_id_and_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_security_audit_events_on_id_and_household_id ON public.security_audit_events USING btree (id, household_id);


--
-- Name: index_storage_migration_runs_on_run_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_storage_migration_runs_on_run_id ON public.storage_migration_runs USING btree (run_id);


--
-- Name: index_support_access_sessions_for_expiry_processing; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_support_access_sessions_for_expiry_processing ON public.support_access_sessions USING btree (ended_at, expired_at, expires_at);


--
-- Name: index_support_access_sessions_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_support_access_sessions_on_household_id ON public.support_access_sessions USING btree (household_id);


--
-- Name: index_support_access_sessions_on_household_id_and_expires_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_support_access_sessions_on_household_id_and_expires_at ON public.support_access_sessions USING btree (household_id, expires_at);


--
-- Name: index_support_access_sessions_on_platform_admin_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_support_access_sessions_on_platform_admin_id ON public.support_access_sessions USING btree (platform_admin_id);


--
-- Name: index_users_on_email_address; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_users_on_email_address ON public.users USING btree (email_address);


--
-- Name: index_users_on_person_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE UNIQUE INDEX index_users_on_person_id ON public.users USING btree (person_id);


--
-- Name: index_versions_on_actor_membership_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_actor_membership_id ON public.versions USING btree (actor_membership_id);


--
-- Name: index_versions_on_created_at; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_created_at ON public.versions USING btree (created_at);


--
-- Name: index_versions_on_event; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_event ON public.versions USING btree (event);


--
-- Name: index_versions_on_household_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_household_id ON public.versions USING btree (household_id);


--
-- Name: index_versions_on_item_type_and_item_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_item_type_and_item_id ON public.versions USING btree (item_type, item_id);


--
-- Name: index_versions_on_request_id; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_request_id ON public.versions USING btree (request_id);


--
-- Name: index_versions_on_whodunnit; Type: INDEX; Schema: public; Owner: med_tracker_owner
--

CREATE INDEX index_versions_on_whodunnit ON public.versions USING btree (whodunnit);


--
-- Name: security_audit_events append_security_events_to_audit_ledger; Type: TRIGGER; Schema: public; Owner: med_tracker_owner
--

CREATE TRIGGER append_security_events_to_audit_ledger AFTER INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.audit_capture_source_row();


--
-- Name: versions append_versions_to_audit_ledger; Type: TRIGGER; Schema: public; Owner: med_tracker_owner
--

CREATE TRIGGER append_versions_to_audit_ledger AFTER INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.audit_capture_source_row();


--
-- Name: carer_relationships fk_carer_relationships_carer_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_carer_relationships_carer_household FOREIGN KEY (carer_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: carer_relationships fk_carer_relationships_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_carer_relationships_household FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: carer_relationships fk_carer_relationships_patient_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_carer_relationships_patient_household FOREIGN KEY (patient_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: dosages fk_dosages_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.dosages
    ADD CONSTRAINT fk_dosages_medication_id_household FOREIGN KEY (medication_id, household_id) REFERENCES public.medications(id, household_id);


--
-- Name: medication_dose_occurrences fk_dose_occurrence_medication_take_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_dose_occurrence_medication_take_id_household FOREIGN KEY (medication_take_id, household_id) REFERENCES public.medication_takes(id, household_id);


--
-- Name: medication_dose_occurrences fk_dose_occurrence_person_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_dose_occurrence_person_medication_id_household FOREIGN KEY (person_medication_id, household_id) REFERENCES public.person_medications(id, household_id);


--
-- Name: medication_dose_occurrences fk_dose_occurrence_resolved_by_membership_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_dose_occurrence_resolved_by_membership_id_household FOREIGN KEY (resolved_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id);


--
-- Name: medication_dose_occurrences fk_dose_occurrence_schedule_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_dose_occurrence_schedule_id_household FOREIGN KEY (schedule_id, household_id) REFERENCES public.schedules(id, household_id);


--
-- Name: health_event_medications fk_health_event_medications_health_event_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications
    ADD CONSTRAINT fk_health_event_medications_health_event_id_household FOREIGN KEY (health_event_id, household_id) REFERENCES public.health_events(id, household_id);


--
-- Name: health_event_medications fk_health_event_medications_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications
    ADD CONSTRAINT fk_health_event_medications_medication_id_household FOREIGN KEY (medication_id, household_id) REFERENCES public.medications(id, household_id);


--
-- Name: health_events fk_health_events_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_events
    ADD CONSTRAINT fk_health_events_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: household_invitation_grants fk_household_invitation_grants_invitation_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants
    ADD CONSTRAINT fk_household_invitation_grants_invitation_household FOREIGN KEY (household_invitation_id, household_id) REFERENCES public.household_invitations(id, household_id);


--
-- Name: household_invitation_grants fk_household_invitation_grants_person_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants
    ADD CONSTRAINT fk_household_invitation_grants_person_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: household_memberships fk_household_memberships_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_memberships
    ADD CONSTRAINT fk_household_memberships_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: location_memberships fk_location_memberships_location_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_location_memberships_location_id_household FOREIGN KEY (location_id, household_id) REFERENCES public.locations(id, household_id);


--
-- Name: location_memberships fk_location_memberships_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_location_memberships_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: medication_pause_periods fk_med_pause_periods_person_medication_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_person_medication_household FOREIGN KEY (person_medication_id, household_id) REFERENCES public.person_medications(id, household_id) NOT VALID;


--
-- Name: medication_pause_periods fk_med_pause_periods_recorded_actor_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_recorded_actor_household FOREIGN KEY (recorded_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id) NOT VALID;


--
-- Name: medication_pause_periods fk_med_pause_periods_resumed_actor_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_resumed_actor_household FOREIGN KEY (resumed_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id) NOT VALID;


--
-- Name: medication_pause_periods fk_med_pause_periods_schedule_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_schedule_household FOREIGN KEY (schedule_id, household_id) REFERENCES public.schedules(id, household_id) NOT VALID;


--
-- Name: medication_takes fk_medication_takes_person_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_medication_takes_person_medication_id_household FOREIGN KEY (person_medication_id, household_id) REFERENCES public.person_medications(id, household_id);


--
-- Name: medication_takes fk_medication_takes_schedule_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_medication_takes_schedule_id_household FOREIGN KEY (schedule_id, household_id) REFERENCES public.schedules(id, household_id);


--
-- Name: medication_takes fk_medication_takes_taken_from_location_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_medication_takes_taken_from_location_id_household FOREIGN KEY (taken_from_location_id, household_id) REFERENCES public.locations(id, household_id);


--
-- Name: medication_takes fk_medication_takes_taken_from_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_medication_takes_taken_from_medication_id_household FOREIGN KEY (taken_from_medication_id, household_id) REFERENCES public.medications(id, household_id);


--
-- Name: medications fk_medications_created_by_membership_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT fk_medications_created_by_membership_id_household FOREIGN KEY (created_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id);


--
-- Name: medications fk_medications_location_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT fk_medications_location_id_household FOREIGN KEY (location_id, household_id) REFERENCES public.locations(id, household_id);


--
-- Name: notification_events fk_notification_events_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_events
    ADD CONSTRAINT fk_notification_events_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: notification_preferences fk_notification_preferences_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT fk_notification_preferences_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: person_access_grants fk_person_access_grants_carer_relationship_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_person_access_grants_carer_relationship_household FOREIGN KEY (carer_relationship_id, household_id) REFERENCES public.carer_relationships(id, household_id);


--
-- Name: person_access_grants fk_person_access_grants_granted_by_membership_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_person_access_grants_granted_by_membership_id_household FOREIGN KEY (granted_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id);


--
-- Name: person_access_grants fk_person_access_grants_household_membership_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_person_access_grants_household_membership_id_household FOREIGN KEY (household_membership_id, household_id) REFERENCES public.household_memberships(id, household_id);


--
-- Name: person_access_grants fk_person_access_grants_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_person_access_grants_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: person_medications fk_person_medications_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_person_medications_medication_id_household FOREIGN KEY (medication_id, household_id) REFERENCES public.medications(id, household_id);


--
-- Name: person_medications fk_person_medications_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_person_medications_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: person_medications fk_person_medications_source_dosage_option_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_person_medications_source_dosage_option_id_household FOREIGN KEY (source_dosage_option_id, household_id) REFERENCES public.dosages(id, household_id);


--
-- Name: health_event_medications fk_rails_008d37b889; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications
    ADD CONSTRAINT fk_rails_008d37b889 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: health_events fk_rails_00db5e06d0; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_events
    ADD CONSTRAINT fk_rails_00db5e06d0 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: household_retention_holds fk_rails_03ae12478f; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_retention_holds
    ADD CONSTRAINT fk_rails_03ae12478f FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: carer_relationships fk_rails_05ed1c349b; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_rails_05ed1c349b FOREIGN KEY (patient_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: api_sessions fk_rails_0f321a0531; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_sessions
    ADD CONSTRAINT fk_rails_0f321a0531 FOREIGN KEY (household_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: api_sessions fk_rails_1018073a20; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_sessions
    ADD CONSTRAINT fk_rails_1018073a20 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: medication_pause_periods fk_rails_13d7545ba0; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_rails_13d7545ba0 FOREIGN KEY (recorded_by_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: versions fk_rails_15b046bfbe; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.versions
    ADD CONSTRAINT fk_rails_15b046bfbe FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: oauth_grants fk_rails_1751f12354; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants
    ADD CONSTRAINT fk_rails_1751f12354 FOREIGN KEY (person_id) REFERENCES public.people(id);


--
-- Name: medication_review_prompts fk_rails_1789ad1477; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_1789ad1477 FOREIGN KEY (reviewed_by_membership_id) REFERENCES public.household_memberships(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: locations fk_rails_19c49f71b4; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT fk_rails_19c49f71b4 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: person_medications fk_rails_1a6ff9b7fd; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_rails_1a6ff9b7fd FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: nhs_dmd_trade_families fk_rails_1ba042dadc; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_trade_families
    ADD CONSTRAINT fk_rails_1ba042dadc FOREIGN KEY (trade_family_group_id) REFERENCES public.nhs_dmd_trade_family_groups(id);


--
-- Name: notification_events fk_rails_1e2df17b6b; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_events
    ADD CONSTRAINT fk_rails_1e2df17b6b FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: medications fk_rails_1f0c478552; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT fk_rails_1f0c478552 FOREIGN KEY (created_by_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: oauth_applications fk_rails_211c1cecac; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_applications
    ADD CONSTRAINT fk_rails_211c1cecac FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: account_webauthn_keys fk_rails_2586b16017; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_keys
    ADD CONSTRAINT fk_rails_2586b16017 FOREIGN KEY (account_id) REFERENCES public.accounts(id) ON DELETE CASCADE;


--
-- Name: audit_export_deliveries fk_rails_27447b69f8; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_export_deliveries
    ADD CONSTRAINT fk_rails_27447b69f8 FOREIGN KEY (audit_ledger_entry_id) REFERENCES public.audit_ledger_entries(id);


--
-- Name: location_memberships fk_rails_280552daaa; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_rails_280552daaa FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: audit_ledger_entries fk_rails_28fa9af012; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_ledger_entries
    ADD CONSTRAINT fk_rails_28fa9af012 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: account_login_failures fk_rails_2d519987a4; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_login_failures
    ADD CONSTRAINT fk_rails_2d519987a4 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: api_idempotency_keys fk_rails_35043a57c0; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_idempotency_keys
    ADD CONSTRAINT fk_rails_35043a57c0 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: api_household_selection_grants fk_rails_3adf622f08; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_household_selection_grants
    ADD CONSTRAINT fk_rails_3adf622f08 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: medication_pause_periods fk_rails_3d02d8db1e; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_rails_3d02d8db1e FOREIGN KEY (person_medication_id) REFERENCES public.person_medications(id);


--
-- Name: medication_takes fk_rails_3db380435a; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_3db380435a FOREIGN KEY (taken_from_location_id) REFERENCES public.locations(id);


--
-- Name: nhs_dmd_amp_trade_families fk_rails_3dd9aab894; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.nhs_dmd_amp_trade_families
    ADD CONSTRAINT fk_rails_3dd9aab894 FOREIGN KEY (trade_family_id) REFERENCES public.nhs_dmd_trade_families(id);


--
-- Name: oauth_grants fk_rails_3e095b0b7e; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants
    ADD CONSTRAINT fk_rails_3e095b0b7e FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: medication_review_prompts fk_rails_417ee86956; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_417ee86956 FOREIGN KEY (evidence_record_id) REFERENCES public.medication_review_evidence_records(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: schedules fk_rails_420e071daa; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_rails_420e071daa FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: push_subscriptions fk_rails_42e723e9f9; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.push_subscriptions
    ADD CONSTRAINT fk_rails_42e723e9f9 FOREIGN KEY (account_id) REFERENCES public.accounts(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: api_change_events fk_rails_448c435863; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_change_events
    ADD CONSTRAINT fk_rails_448c435863 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: health_event_medications fk_rails_4b7b1ea600; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications
    ADD CONSTRAINT fk_rails_4b7b1ea600 FOREIGN KEY (medication_id) REFERENCES public.medications(id);


--
-- Name: medication_pause_periods fk_rails_4cd0a97e69; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_rails_4cd0a97e69 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: medication_dose_occurrences fk_rails_4e7357c544; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_rails_4e7357c544 FOREIGN KEY (resolved_by_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: person_medications fk_rails_50c1621a30; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_rails_50c1621a30 FOREIGN KEY (medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: api_tombstones fk_rails_5316070ef5; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_tombstones
    ADD CONSTRAINT fk_rails_5316070ef5 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: person_access_grants fk_rails_53df2ee65d; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_rails_53df2ee65d FOREIGN KEY (person_id) REFERENCES public.people(id);


--
-- Name: household_exports fk_rails_572f0022f4; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_exports
    ADD CONSTRAINT fk_rails_572f0022f4 FOREIGN KEY (requested_by_account_id) REFERENCES public.accounts(id);


--
-- Name: household_purge_runs fk_rails_599c66dae5; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_purge_runs
    ADD CONSTRAINT fk_rails_599c66dae5 FOREIGN KEY (requested_by_account_id) REFERENCES public.accounts(id);


--
-- Name: api_app_tokens fk_rails_5a60bf71ce; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_app_tokens
    ADD CONSTRAINT fk_rails_5a60bf71ce FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: medication_dose_occurrences fk_rails_6219f3f23e; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_rails_6219f3f23e FOREIGN KEY (medication_take_id) REFERENCES public.medication_takes(id);


--
-- Name: dosages fk_rails_649e7a6bdb; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.dosages
    ADD CONSTRAINT fk_rails_649e7a6bdb FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: schedules fk_rails_6a2170127e; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_rails_6a2170127e FOREIGN KEY (medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: notification_preferences fk_rails_6b4e5876ef; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT fk_rails_6b4e5876ef FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: medication_review_prompts fk_rails_6dd665ced2; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_6dd665ced2 FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: medication_takes fk_rails_72c4e175ed; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_72c4e175ed FOREIGN KEY (person_medication_id) REFERENCES public.person_medications(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: account_webauthn_user_ids fk_rails_742fee62ad; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_webauthn_user_ids
    ADD CONSTRAINT fk_rails_742fee62ad FOREIGN KEY (account_id) REFERENCES public.accounts(id) ON DELETE CASCADE;


--
-- Name: security_audit_events fk_rails_75bd8b3918; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.security_audit_events
    ADD CONSTRAINT fk_rails_75bd8b3918 FOREIGN KEY (actor_account_id) REFERENCES public.accounts(id);


--
-- Name: person_medications fk_rails_78f10950ce; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_rails_78f10950ce FOREIGN KEY (source_dosage_option_id) REFERENCES public.dosages(id);


--
-- Name: medication_dose_occurrences fk_rails_7bb88a451f; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_rails_7bb88a451f FOREIGN KEY (person_medication_id) REFERENCES public.person_medications(id);


--
-- Name: account_otp_keys fk_rails_823f8d7a81; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_otp_keys
    ADD CONSTRAINT fk_rails_823f8d7a81 FOREIGN KEY (id) REFERENCES public.accounts(id);


--
-- Name: people fk_rails_829e856eff; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.people
    ADD CONSTRAINT fk_rails_829e856eff FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: medication_pause_periods fk_rails_82a2dd6e6d; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_rails_82a2dd6e6d FOREIGN KEY (schedule_id) REFERENCES public.schedules(id);


--
-- Name: medication_review_prompts fk_rails_83e19c8bb8; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_83e19c8bb8 FOREIGN KEY (primary_medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: household_memberships fk_rails_83f9a515c3; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_memberships
    ADD CONSTRAINT fk_rails_83f9a515c3 FOREIGN KEY (person_id) REFERENCES public.people(id);


--
-- Name: platform_admins fk_rails_84d8e101dd; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.platform_admins
    ADD CONSTRAINT fk_rails_84d8e101dd FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: medication_pause_periods fk_rails_8513a75db1; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_rails_8513a75db1 FOREIGN KEY (resumed_by_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: schedules fk_rails_87ac9a7f10; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_rails_87ac9a7f10 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: schedules fk_rails_88300b88fe; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_rails_88300b88fe FOREIGN KEY (source_dosage_option_id) REFERENCES public.dosages(id);


--
-- Name: account_identities fk_rails_8868aa7ac0; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_identities
    ADD CONSTRAINT fk_rails_8868aa7ac0 FOREIGN KEY (account_id) REFERENCES public.accounts(id) ON DELETE CASCADE;


--
-- Name: api_tombstones fk_rails_90265af5fd; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_tombstones
    ADD CONSTRAINT fk_rails_90265af5fd FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: household_retention_holds fk_rails_932cf78829; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_retention_holds
    ADD CONSTRAINT fk_rails_932cf78829 FOREIGN KEY (released_by_account_id) REFERENCES public.accounts(id);


--
-- Name: medication_review_prompts fk_rails_93516870c8; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_93516870c8 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: support_access_sessions fk_rails_95551392d9; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.support_access_sessions
    ADD CONSTRAINT fk_rails_95551392d9 FOREIGN KEY (platform_admin_id) REFERENCES public.platform_admins(id);


--
-- Name: active_storage_variant_records fk_rails_993965df05; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_variant_records
    ADD CONSTRAINT fk_rails_993965df05 FOREIGN KEY (blob_id) REFERENCES public.active_storage_blobs(id);


--
-- Name: api_idempotency_keys fk_rails_9f635195b2; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_idempotency_keys
    ADD CONSTRAINT fk_rails_9f635195b2 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: person_access_grants fk_rails_a04c18169e; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_rails_a04c18169e FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: api_app_tokens fk_rails_a0a56faf23; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_app_tokens
    ADD CONSTRAINT fk_rails_a0a56faf23 FOREIGN KEY (household_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: notification_preferences fk_rails_a1af1e497f; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT fk_rails_a1af1e497f FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: audit_checkpoints fk_rails_a25b9abfde; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_checkpoints
    ADD CONSTRAINT fk_rails_a25b9abfde FOREIGN KEY (audit_signing_key_id) REFERENCES public.audit_signing_keys(id);


--
-- Name: people fk_rails_a2afb3d410; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.people
    ADD CONSTRAINT fk_rails_a2afb3d410 FOREIGN KEY (account_id) REFERENCES public.accounts(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: audit_chain_heads fk_rails_a311021dea; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_chain_heads
    ADD CONSTRAINT fk_rails_a311021dea FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: api_idempotency_keys fk_rails_a31f64e500; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_idempotency_keys
    ADD CONSTRAINT fk_rails_a31f64e500 FOREIGN KEY (api_session_id) REFERENCES public.api_sessions(id);


--
-- Name: dosages fk_rails_a34bc544d1; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.dosages
    ADD CONSTRAINT fk_rails_a34bc544d1 FOREIGN KEY (medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: person_medications fk_rails_a435932ea2; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_rails_a435932ea2 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: api_idempotency_keys fk_rails_a4c5b33ad6; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_idempotency_keys
    ADD CONSTRAINT fk_rails_a4c5b33ad6 FOREIGN KEY (api_app_token_id) REFERENCES public.api_app_tokens(id);


--
-- Name: household_invitation_grants fk_rails_a8444d0c26; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants
    ADD CONSTRAINT fk_rails_a8444d0c26 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: api_change_events fk_rails_a87c7f4df1; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_change_events
    ADD CONSTRAINT fk_rails_a87c7f4df1 FOREIGN KEY (household_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: notification_events fk_rails_ac049150f6; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.notification_events
    ADD CONSTRAINT fk_rails_ac049150f6 FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: medication_takes fk_rails_ad685ca12a; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_ad685ca12a FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: oauth_grants fk_rails_af16ab68d1; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants
    ADD CONSTRAINT fk_rails_af16ab68d1 FOREIGN KEY (household_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: household_exports fk_rails_af46b6b2a8; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_exports
    ADD CONSTRAINT fk_rails_af46b6b2a8 FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: api_change_events fk_rails_af53a77073; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_change_events
    ADD CONSTRAINT fk_rails_af53a77073 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: household_purge_runs fk_rails_af57a983cb; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_purge_runs
    ADD CONSTRAINT fk_rails_af57a983cb FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: carer_relationships fk_rails_b2f8dc4c56; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_rails_b2f8dc4c56 FOREIGN KEY (carer_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: audit_checkpoints fk_rails_b436553447; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_checkpoints
    ADD CONSTRAINT fk_rails_b436553447 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: location_memberships fk_rails_b5fe564bc0; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_rails_b5fe564bc0 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: households fk_rails_b9ce8e6e8e; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.households
    ADD CONSTRAINT fk_rails_b9ce8e6e8e FOREIGN KEY (created_by_account_id) REFERENCES public.accounts(id);


--
-- Name: medication_takes fk_rails_bac9dc4c0c; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_bac9dc4c0c FOREIGN KEY (schedule_id) REFERENCES public.schedules(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: security_audit_events fk_rails_bc150fd06f; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.security_audit_events
    ADD CONSTRAINT fk_rails_bc150fd06f FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: health_event_medications fk_rails_be17586637; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_event_medications
    ADD CONSTRAINT fk_rails_be17586637 FOREIGN KEY (health_event_id) REFERENCES public.health_events(id);


--
-- Name: household_retention_holds fk_rails_c12e6f729d; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_retention_holds
    ADD CONSTRAINT fk_rails_c12e6f729d FOREIGN KEY (approved_by_account_id) REFERENCES public.accounts(id);


--
-- Name: medication_dose_occurrences fk_rails_c3627bca9a; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_rails_c3627bca9a FOREIGN KEY (schedule_id) REFERENCES public.schedules(id);


--
-- Name: active_storage_attachments fk_rails_c3b3935057; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_attachments
    ADD CONSTRAINT fk_rails_c3b3935057 FOREIGN KEY (blob_id) REFERENCES public.active_storage_blobs(id);


--
-- Name: api_tombstones fk_rails_c453d62529; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.api_tombstones
    ADD CONSTRAINT fk_rails_c453d62529 FOREIGN KEY (household_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: account_password_reset_keys fk_rails_c5c7a8fb9b; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_password_reset_keys
    ADD CONSTRAINT fk_rails_c5c7a8fb9b FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: active_storage_attachments fk_rails_c6a1fbd2bf; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.active_storage_attachments
    ADD CONSTRAINT fk_rails_c6a1fbd2bf FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: audit_export_deliveries fk_rails_c9deefabbe; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.audit_export_deliveries
    ADD CONSTRAINT fk_rails_c9deefabbe FOREIGN KEY (audit_checkpoint_id) REFERENCES public.audit_checkpoints(id);


--
-- Name: household_invitation_grants fk_rails_cbf5dd79c9; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants
    ADD CONSTRAINT fk_rails_cbf5dd79c9 FOREIGN KEY (household_invitation_id) REFERENCES public.household_invitations(id);


--
-- Name: account_active_session_keys fk_rails_cdedf5be2c; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_active_session_keys
    ADD CONSTRAINT fk_rails_cdedf5be2c FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: health_events fk_rails_ce4cdc7516; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.health_events
    ADD CONSTRAINT fk_rails_ce4cdc7516 FOREIGN KEY (person_id) REFERENCES public.people(id);


--
-- Name: household_invitation_grants fk_rails_d2daa8499a; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitation_grants
    ADD CONSTRAINT fk_rails_d2daa8499a FOREIGN KEY (person_id) REFERENCES public.people(id);


--
-- Name: medications fk_rails_d4b5830986; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT fk_rails_d4b5830986 FOREIGN KEY (location_id) REFERENCES public.locations(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: oauth_grants fk_rails_d5addd7cc9; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants
    ADD CONSTRAINT fk_rails_d5addd7cc9 FOREIGN KEY (oauth_application_id) REFERENCES public.oauth_applications(id);


--
-- Name: medication_review_prompts fk_rails_d7ad4ad8a3; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_d7ad4ad8a3 FOREIGN KEY (interacting_medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: account_lockouts fk_rails_d7d088ec4c; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_lockouts
    ADD CONSTRAINT fk_rails_d7d088ec4c FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: support_access_sessions fk_rails_d8b00bbe35; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.support_access_sessions
    ADD CONSTRAINT fk_rails_d8b00bbe35 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: household_invitations fk_rails_dcc0aec101; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitations
    ADD CONSTRAINT fk_rails_dcc0aec101 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: person_access_grants fk_rails_de619a6e71; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_rails_de619a6e71 FOREIGN KEY (household_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: medication_dose_occurrences fk_rails_dee524eb5b; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_dose_occurrences
    ADD CONSTRAINT fk_rails_dee524eb5b FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: medication_takes fk_rails_e0faca3a58; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_e0faca3a58 FOREIGN KEY (taken_from_medication_id) REFERENCES public.medications(id);


--
-- Name: native_device_tokens fk_rails_e3e4bfb5dc; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.native_device_tokens
    ADD CONSTRAINT fk_rails_e3e4bfb5dc FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: account_login_change_keys fk_rails_e65835dcbc; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_login_change_keys
    ADD CONSTRAINT fk_rails_e65835dcbc FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: location_memberships fk_rails_e70491d63b; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_rails_e70491d63b FOREIGN KEY (location_id) REFERENCES public.locations(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: household_memberships fk_rails_e7dc011d7c; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_memberships
    ADD CONSTRAINT fk_rails_e7dc011d7c FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: account_verification_keys fk_rails_eb691e7e17; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_verification_keys
    ADD CONSTRAINT fk_rails_eb691e7e17 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: account_recovery_codes fk_rails_ef43948844; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_recovery_codes
    ADD CONSTRAINT fk_rails_ef43948844 FOREIGN KEY (id) REFERENCES public.accounts(id);


--
-- Name: household_memberships fk_rails_f11dbb5a09; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_memberships
    ADD CONSTRAINT fk_rails_f11dbb5a09 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: medications fk_rails_f34d593da6; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT fk_rails_f34d593da6 FOREIGN KEY (household_id) REFERENCES public.households(id);


--
-- Name: household_invitations fk_rails_f7412b2e4d; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.household_invitations
    ADD CONSTRAINT fk_rails_f7412b2e4d FOREIGN KEY (invited_by_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: account_remember_keys fk_rails_fa67029c99; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.account_remember_keys
    ADD CONSTRAINT fk_rails_fa67029c99 FOREIGN KEY (account_id) REFERENCES public.accounts(id);


--
-- Name: users fk_rails_fa67535741; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT fk_rails_fa67535741 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: person_access_grants fk_rails_fcbf4f0da8; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.person_access_grants
    ADD CONSTRAINT fk_rails_fcbf4f0da8 FOREIGN KEY (granted_by_membership_id) REFERENCES public.household_memberships(id);


--
-- Name: medication_review_prompts fk_review_prompts_interacting_medication_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_interacting_medication_household FOREIGN KEY (interacting_medication_id, household_id) REFERENCES public.medications(id, household_id) NOT VALID;


--
-- Name: medication_review_prompts fk_review_prompts_person_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_person_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id) NOT VALID;


--
-- Name: medication_review_prompts fk_review_prompts_primary_medication_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_primary_medication_household FOREIGN KEY (primary_medication_id, household_id) REFERENCES public.medications(id, household_id) NOT VALID;


--
-- Name: medication_review_prompts fk_review_prompts_reviewer_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_reviewer_household FOREIGN KEY (reviewed_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id) NOT VALID;


--
-- Name: schedules fk_schedules_medication_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_schedules_medication_id_household FOREIGN KEY (medication_id, household_id) REFERENCES public.medications(id, household_id);


--
-- Name: schedules fk_schedules_person_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_schedules_person_id_household FOREIGN KEY (person_id, household_id) REFERENCES public.people(id, household_id);


--
-- Name: schedules fk_schedules_source_dosage_option_id_household; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_schedules_source_dosage_option_id_household FOREIGN KEY (source_dosage_option_id, household_id) REFERENCES public.dosages(id, household_id);


--
-- Name: oauth_grants oauth_grant_application_kind; Type: FK CONSTRAINT; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE ONLY public.oauth_grants
    ADD CONSTRAINT oauth_grant_application_kind FOREIGN KEY (oauth_application_id, client_kind) REFERENCES public.oauth_applications(id, client_kind);


--
-- Name: active_storage_attachments; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.active_storage_attachments ENABLE ROW LEVEL SECURITY;

--
-- Name: api_change_events; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.api_change_events ENABLE ROW LEVEL SECURITY;

--
-- Name: api_idempotency_keys; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.api_idempotency_keys ENABLE ROW LEVEL SECURITY;

--
-- Name: api_tombstones; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.api_tombstones ENABLE ROW LEVEL SECURITY;

--
-- Name: security_audit_events audit_verifier_complete_visibility; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY audit_verifier_complete_visibility ON public.security_audit_events FOR SELECT TO med_tracker_audit_verifier USING (true);


--
-- Name: carer_relationships; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.carer_relationships ENABLE ROW LEVEL SECURITY;

--
-- Name: dosages; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.dosages ENABLE ROW LEVEL SECURITY;

--
-- Name: health_event_medications; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.health_event_medications ENABLE ROW LEVEL SECURITY;

--
-- Name: health_events; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.health_events ENABLE ROW LEVEL SECURITY;

--
-- Name: household_exports; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.household_exports ENABLE ROW LEVEL SECURITY;

--
-- Name: household_invitation_grants; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.household_invitation_grants ENABLE ROW LEVEL SECURITY;

--
-- Name: household_invitations; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.household_invitations ENABLE ROW LEVEL SECURITY;

--
-- Name: household_memberships; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.household_memberships ENABLE ROW LEVEL SECURITY;

--
-- Name: household_retention_holds; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.household_retention_holds ENABLE ROW LEVEL SECURITY;

--
-- Name: active_storage_attachments household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.active_storage_attachments USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: api_change_events household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.api_change_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: api_idempotency_keys household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.api_idempotency_keys USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: api_tombstones household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.api_tombstones USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: carer_relationships household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.carer_relationships USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: dosages household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.dosages USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: health_event_medications household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.health_event_medications USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: health_events household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.health_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: household_exports household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.household_exports USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: household_invitation_grants household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.household_invitation_grants USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: household_invitations household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.household_invitations USING (((household_id = med_tracker.current_household_id()) OR (((token_digest)::text = med_tracker.current_invitation_token_digest()) AND (accepted_at IS NULL) AND (revoked_at IS NULL) AND (expires_at > CURRENT_TIMESTAMP)))) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: household_memberships household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.household_memberships USING (((household_id = med_tracker.current_household_id()) OR (account_id = med_tracker.current_account_id()))) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: household_retention_holds household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.household_retention_holds USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: location_memberships household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.location_memberships USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: locations household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.locations USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: medication_dose_occurrences household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.medication_dose_occurrences USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: medication_pause_periods household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.medication_pause_periods USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: medication_review_prompts household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.medication_review_prompts USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: medication_takes household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.medication_takes USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: medications household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.medications USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: notification_events household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.notification_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: notification_preferences household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.notification_preferences USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: people household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.people USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: person_access_grants household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.person_access_grants USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: person_medications household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.person_medications USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: schedules household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.schedules USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: security_audit_events household_tenant_isolation; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY household_tenant_isolation ON public.security_audit_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


--
-- Name: location_memberships; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.location_memberships ENABLE ROW LEVEL SECURITY;

--
-- Name: locations; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.locations ENABLE ROW LEVEL SECURITY;

--
-- Name: medication_dose_occurrences; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.medication_dose_occurrences ENABLE ROW LEVEL SECURITY;

--
-- Name: medication_pause_periods; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.medication_pause_periods ENABLE ROW LEVEL SECURITY;

--
-- Name: medication_review_prompts; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.medication_review_prompts ENABLE ROW LEVEL SECURITY;

--
-- Name: medication_takes; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.medication_takes ENABLE ROW LEVEL SECURITY;

--
-- Name: medications; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.medications ENABLE ROW LEVEL SECURITY;

--
-- Name: notification_events; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.notification_events ENABLE ROW LEVEL SECURITY;

--
-- Name: notification_preferences; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.notification_preferences ENABLE ROW LEVEL SECURITY;

--
-- Name: people; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.people ENABLE ROW LEVEL SECURITY;

--
-- Name: people people_account_login_lookup; Type: POLICY; Schema: public; Owner: med_tracker_owner
--

CREATE POLICY people_account_login_lookup ON public.people FOR SELECT TO med_tracker_app USING ((account_id IS NOT NULL));


--
-- Name: person_access_grants; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.person_access_grants ENABLE ROW LEVEL SECURITY;

--
-- Name: person_medications; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.person_medications ENABLE ROW LEVEL SECURITY;

--
-- Name: schedules; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.schedules ENABLE ROW LEVEL SECURITY;

--
-- Name: security_audit_events; Type: ROW SECURITY; Schema: public; Owner: med_tracker_owner
--

ALTER TABLE public.security_audit_events ENABLE ROW LEVEL SECURITY;

--
-- Name: SCHEMA med_tracker; Type: ACL; Schema: -; Owner: medtracker
--

GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner;
GRANT USAGE ON SCHEMA med_tracker TO med_tracker_app;


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: pg_database_owner
--

GRANT ALL ON SCHEMA public TO med_tracker_owner;
GRANT USAGE ON SCHEMA public TO med_tracker_app;
GRANT USAGE ON SCHEMA public TO med_tracker_audit_exporter;
GRANT USAGE ON SCHEMA public TO med_tracker_audit_verifier;


--
-- Name: FUNCTION current_account_id(); Type: ACL; Schema: med_tracker; Owner: med_tracker_owner
--

GRANT ALL ON FUNCTION med_tracker.current_account_id() TO med_tracker_app;


--
-- Name: FUNCTION current_household_id(); Type: ACL; Schema: med_tracker; Owner: med_tracker_owner
--

GRANT ALL ON FUNCTION med_tracker.current_household_id() TO med_tracker_app;


--
-- Name: FUNCTION current_invitation_token_digest(); Type: ACL; Schema: med_tracker; Owner: med_tracker_owner
--

GRANT ALL ON FUNCTION med_tracker.current_invitation_token_digest() TO med_tracker_app;


--
-- Name: FUNCTION current_membership_id(); Type: ACL; Schema: med_tracker; Owner: med_tracker_owner
--

GRANT ALL ON FUNCTION med_tracker.current_membership_id() TO med_tracker_app;


--
-- Name: FUNCTION purge_medication_takes(p_household_id bigint); Type: ACL; Schema: med_tracker; Owner: med_tracker_owner
--

REVOKE ALL ON FUNCTION med_tracker.purge_medication_takes(p_household_id bigint) FROM PUBLIC;
GRANT ALL ON FUNCTION med_tracker.purge_medication_takes(p_household_id bigint) TO med_tracker_app;


--
-- Name: FUNCTION audit_append_ledger_entry(p_source_table text, p_source_id bigint, p_household_id bigint, p_source_payload jsonb, p_occurred_at timestamp with time zone); Type: ACL; Schema: public; Owner: med_tracker_owner
--

REVOKE ALL ON FUNCTION public.audit_append_ledger_entry(p_source_table text, p_source_id bigint, p_household_id bigint, p_source_payload jsonb, p_occurred_at timestamp with time zone) FROM PUBLIC;


--
-- Name: FUNCTION audit_capture_source_row(); Type: ACL; Schema: public; Owner: med_tracker_owner
--

REVOKE ALL ON FUNCTION public.audit_capture_source_row() FROM PUBLIC;


--
-- Name: FUNCTION audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text); Type: ACL; Schema: public; Owner: med_tracker_owner
--

REVOKE ALL ON FUNCTION public.audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text) TO med_tracker_audit_exporter;


--
-- Name: TABLE account_active_session_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_active_session_keys TO med_tracker_app;


--
-- Name: TABLE account_identities; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_identities TO med_tracker_app;


--
-- Name: SEQUENCE account_identities_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_identities_id_seq TO med_tracker_app;


--
-- Name: TABLE account_lockouts; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_lockouts TO med_tracker_app;


--
-- Name: SEQUENCE account_lockouts_account_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_lockouts_account_id_seq TO med_tracker_app;


--
-- Name: TABLE account_login_change_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_login_change_keys TO med_tracker_app;


--
-- Name: TABLE account_login_failures; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_login_failures TO med_tracker_app;


--
-- Name: SEQUENCE account_login_failures_account_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_login_failures_account_id_seq TO med_tracker_app;


--
-- Name: TABLE account_otp_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_otp_keys TO med_tracker_app;


--
-- Name: SEQUENCE account_otp_keys_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_otp_keys_id_seq TO med_tracker_app;


--
-- Name: TABLE account_password_reset_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_password_reset_keys TO med_tracker_app;


--
-- Name: TABLE account_recovery_codes; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_recovery_codes TO med_tracker_app;


--
-- Name: TABLE account_remember_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_remember_keys TO med_tracker_app;


--
-- Name: TABLE account_verification_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_verification_keys TO med_tracker_app;


--
-- Name: TABLE account_webauthn_auth_challenges; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_webauthn_auth_challenges TO med_tracker_app;


--
-- Name: SEQUENCE account_webauthn_auth_challenges_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_webauthn_auth_challenges_id_seq TO med_tracker_app;


--
-- Name: TABLE account_webauthn_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_webauthn_keys TO med_tracker_app;


--
-- Name: SEQUENCE account_webauthn_keys_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_webauthn_keys_id_seq TO med_tracker_app;


--
-- Name: TABLE account_webauthn_user_ids; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_webauthn_user_ids TO med_tracker_app;


--
-- Name: SEQUENCE account_webauthn_user_ids_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.account_webauthn_user_ids_id_seq TO med_tracker_app;


--
-- Name: TABLE accounts; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.accounts TO med_tracker_app;


--
-- Name: SEQUENCE accounts_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.accounts_id_seq TO med_tracker_app;


--
-- Name: TABLE active_storage_attachments; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.active_storage_attachments TO med_tracker_app;


--
-- Name: SEQUENCE active_storage_attachments_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.active_storage_attachments_id_seq TO med_tracker_app;


--
-- Name: TABLE active_storage_blobs; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.active_storage_blobs TO med_tracker_app;


--
-- Name: SEQUENCE active_storage_blobs_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.active_storage_blobs_id_seq TO med_tracker_app;


--
-- Name: TABLE active_storage_variant_records; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.active_storage_variant_records TO med_tracker_app;


--
-- Name: SEQUENCE active_storage_variant_records_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.active_storage_variant_records_id_seq TO med_tracker_app;


--
-- Name: TABLE api_app_tokens; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_app_tokens TO med_tracker_app;


--
-- Name: SEQUENCE api_app_tokens_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.api_app_tokens_id_seq TO med_tracker_app;


--
-- Name: TABLE api_change_events; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_change_events TO med_tracker_app;


--
-- Name: SEQUENCE api_change_events_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.api_change_events_id_seq TO med_tracker_app;


--
-- Name: TABLE api_household_selection_grants; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_household_selection_grants TO med_tracker_app;


--
-- Name: SEQUENCE api_household_selection_grants_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.api_household_selection_grants_id_seq TO med_tracker_app;


--
-- Name: TABLE api_idempotency_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_idempotency_keys TO med_tracker_app;


--
-- Name: SEQUENCE api_idempotency_keys_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.api_idempotency_keys_id_seq TO med_tracker_app;


--
-- Name: TABLE api_sessions; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_sessions TO med_tracker_app;


--
-- Name: SEQUENCE api_sessions_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.api_sessions_id_seq TO med_tracker_app;


--
-- Name: TABLE api_tombstones; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_tombstones TO med_tracker_app;


--
-- Name: SEQUENCE api_tombstones_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.api_tombstones_id_seq TO med_tracker_app;


--
-- Name: TABLE app_settings; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.app_settings TO med_tracker_app;


--
-- Name: SEQUENCE app_settings_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.app_settings_id_seq TO med_tracker_app;


--
-- Name: TABLE audit_chain_heads; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT ON TABLE public.audit_chain_heads TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_chain_heads TO med_tracker_audit_verifier;


--
-- Name: TABLE audit_checkpoints; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT ON TABLE public.audit_checkpoints TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_checkpoints TO med_tracker_audit_verifier;


--
-- Name: TABLE audit_export_deliveries; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,UPDATE ON TABLE public.audit_export_deliveries TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_export_deliveries TO med_tracker_audit_verifier;


--
-- Name: TABLE audit_ledger_entries; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT ON TABLE public.audit_ledger_entries TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_ledger_entries TO med_tracker_audit_verifier;


--
-- Name: TABLE audit_signing_keys; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT ON TABLE public.audit_signing_keys TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_signing_keys TO med_tracker_audit_verifier;


--
-- Name: TABLE barcode_catalog_entries; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.barcode_catalog_entries TO med_tracker_app;


--
-- Name: SEQUENCE barcode_catalog_entries_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.barcode_catalog_entries_id_seq TO med_tracker_app;


--
-- Name: TABLE carer_relationships; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.carer_relationships TO med_tracker_app;


--
-- Name: SEQUENCE carer_relationships_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.carer_relationships_id_seq TO med_tracker_app;


--
-- Name: TABLE dosages; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.dosages TO med_tracker_app;


--
-- Name: SEQUENCE dosages_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.dosages_id_seq TO med_tracker_app;


--
-- Name: TABLE health_event_medications; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.health_event_medications TO med_tracker_app;


--
-- Name: SEQUENCE health_event_medications_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.health_event_medications_id_seq TO med_tracker_app;


--
-- Name: TABLE health_events; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.health_events TO med_tracker_app;


--
-- Name: SEQUENCE health_events_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.health_events_id_seq TO med_tracker_app;


--
-- Name: TABLE household_audit_ledger_entries; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT ON TABLE public.household_audit_ledger_entries TO med_tracker_app;


--
-- Name: TABLE household_exports; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_exports TO med_tracker_app;


--
-- Name: SEQUENCE household_exports_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.household_exports_id_seq TO med_tracker_app;


--
-- Name: TABLE household_invitation_grants; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_invitation_grants TO med_tracker_app;


--
-- Name: SEQUENCE household_invitation_grants_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.household_invitation_grants_id_seq TO med_tracker_app;


--
-- Name: TABLE household_invitations; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_invitations TO med_tracker_app;


--
-- Name: SEQUENCE household_invitations_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.household_invitations_id_seq TO med_tracker_app;


--
-- Name: TABLE household_memberships; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_memberships TO med_tracker_app;


--
-- Name: SEQUENCE household_memberships_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.household_memberships_id_seq TO med_tracker_app;


--
-- Name: TABLE household_purge_runs; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_purge_runs TO med_tracker_app;


--
-- Name: SEQUENCE household_purge_runs_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.household_purge_runs_id_seq TO med_tracker_app;


--
-- Name: TABLE household_retention_holds; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_retention_holds TO med_tracker_app;


--
-- Name: SEQUENCE household_retention_holds_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.household_retention_holds_id_seq TO med_tracker_app;


--
-- Name: TABLE households; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.households TO med_tracker_app;


--
-- Name: SEQUENCE households_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.households_id_seq TO med_tracker_app;


--
-- Name: TABLE location_memberships; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.location_memberships TO med_tracker_app;


--
-- Name: SEQUENCE location_memberships_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.location_memberships_id_seq TO med_tracker_app;


--
-- Name: TABLE locations; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.locations TO med_tracker_app;


--
-- Name: SEQUENCE locations_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.locations_id_seq TO med_tracker_app;


--
-- Name: TABLE medication_dose_occurrences; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_dose_occurrences TO med_tracker_app;


--
-- Name: SEQUENCE medication_dose_occurrences_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medication_dose_occurrences_id_seq TO med_tracker_app;


--
-- Name: TABLE medication_pause_periods; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_pause_periods TO med_tracker_app;


--
-- Name: SEQUENCE medication_pause_periods_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medication_pause_periods_id_seq TO med_tracker_app;


--
-- Name: TABLE medication_review_evidence_records; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_review_evidence_records TO med_tracker_app;


--
-- Name: SEQUENCE medication_review_evidence_records_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medication_review_evidence_records_id_seq TO med_tracker_app;


--
-- Name: TABLE medication_review_evidence_refresh_runs; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_review_evidence_refresh_runs TO med_tracker_app;


--
-- Name: SEQUENCE medication_review_evidence_refresh_runs_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medication_review_evidence_refresh_runs_id_seq TO med_tracker_app;


--
-- Name: TABLE medication_review_prompts; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_review_prompts TO med_tracker_app;


--
-- Name: SEQUENCE medication_review_prompts_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medication_review_prompts_id_seq TO med_tracker_app;


--
-- Name: TABLE medication_takes; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_takes TO med_tracker_app;


--
-- Name: SEQUENCE medication_takes_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medication_takes_id_seq TO med_tracker_app;


--
-- Name: TABLE medications; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medications TO med_tracker_app;


--
-- Name: SEQUENCE medications_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.medications_id_seq TO med_tracker_app;


--
-- Name: TABLE native_device_tokens; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.native_device_tokens TO med_tracker_app;


--
-- Name: SEQUENCE native_device_tokens_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.native_device_tokens_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_amp_trade_families; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_amp_trade_families TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_amp_trade_families_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_amp_trade_families_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_ampp_relationships; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_ampp_relationships TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_ampp_relationships_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_ampp_relationships_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_barcodes; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_barcodes TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_barcodes_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_barcodes_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_imports; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_imports TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_imports_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_imports_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_supplementary_releases; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_supplementary_releases TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_supplementary_releases_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_supplementary_releases_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_trade_families; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_trade_families TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_trade_families_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_trade_families_id_seq TO med_tracker_app;


--
-- Name: TABLE nhs_dmd_trade_family_groups; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_trade_family_groups TO med_tracker_app;


--
-- Name: SEQUENCE nhs_dmd_trade_family_groups_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_trade_family_groups_id_seq TO med_tracker_app;


--
-- Name: TABLE notification_events; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.notification_events TO med_tracker_app;


--
-- Name: SEQUENCE notification_events_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.notification_events_id_seq TO med_tracker_app;


--
-- Name: TABLE notification_preferences; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.notification_preferences TO med_tracker_app;


--
-- Name: SEQUENCE notification_preferences_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.notification_preferences_id_seq TO med_tracker_app;


--
-- Name: TABLE oauth_applications; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.oauth_applications TO med_tracker_app;


--
-- Name: SEQUENCE oauth_applications_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.oauth_applications_id_seq TO med_tracker_app;


--
-- Name: TABLE oauth_grants; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.oauth_grants TO med_tracker_app;


--
-- Name: SEQUENCE oauth_grants_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.oauth_grants_id_seq TO med_tracker_app;


--
-- Name: TABLE people; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.people TO med_tracker_app;


--
-- Name: SEQUENCE people_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.people_id_seq TO med_tracker_app;


--
-- Name: TABLE person_access_grants; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.person_access_grants TO med_tracker_app;


--
-- Name: SEQUENCE person_access_grants_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.person_access_grants_id_seq TO med_tracker_app;


--
-- Name: TABLE person_medications; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.person_medications TO med_tracker_app;


--
-- Name: SEQUENCE person_medications_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.person_medications_id_seq TO med_tracker_app;


--
-- Name: TABLE platform_admins; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.platform_admins TO med_tracker_app;


--
-- Name: SEQUENCE platform_admins_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.platform_admins_id_seq TO med_tracker_app;


--
-- Name: TABLE push_subscriptions; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.push_subscriptions TO med_tracker_app;


--
-- Name: SEQUENCE push_subscriptions_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.push_subscriptions_id_seq TO med_tracker_app;


--
-- Name: TABLE schedules; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.schedules TO med_tracker_app;


--
-- Name: SEQUENCE schedules_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.schedules_id_seq TO med_tracker_app;


--
-- Name: TABLE security_audit_events; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT ON TABLE public.security_audit_events TO med_tracker_app;
GRANT SELECT ON TABLE public.security_audit_events TO med_tracker_audit_verifier;


--
-- Name: SEQUENCE security_audit_events_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.security_audit_events_id_seq TO med_tracker_app;


--
-- Name: TABLE storage_migration_runs; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.storage_migration_runs TO med_tracker_app;


--
-- Name: SEQUENCE storage_migration_runs_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.storage_migration_runs_id_seq TO med_tracker_app;


--
-- Name: TABLE support_access_sessions; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.support_access_sessions TO med_tracker_app;


--
-- Name: SEQUENCE support_access_sessions_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.support_access_sessions_id_seq TO med_tracker_app;


--
-- Name: TABLE users; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.users TO med_tracker_app;


--
-- Name: SEQUENCE users_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.users_id_seq TO med_tracker_app;


--
-- Name: TABLE versions; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,INSERT ON TABLE public.versions TO med_tracker_app;
GRANT SELECT ON TABLE public.versions TO med_tracker_audit_verifier;


--
-- Name: SEQUENCE versions_id_seq; Type: ACL; Schema: public; Owner: med_tracker_owner
--

GRANT SELECT,USAGE ON SEQUENCE public.versions_id_seq TO med_tracker_app;


--
-- Name: DEFAULT PRIVILEGES FOR FUNCTIONS; Type: DEFAULT ACL; Schema: med_tracker; Owner: med_tracker_owner
--

ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA med_tracker GRANT ALL ON FUNCTIONS TO med_tracker_app;


--
-- Name: DEFAULT PRIVILEGES FOR SEQUENCES; Type: DEFAULT ACL; Schema: public; Owner: med_tracker_owner
--

ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public GRANT SELECT,USAGE ON SEQUENCES TO med_tracker_app;


--
-- Name: DEFAULT PRIVILEGES FOR TABLES; Type: DEFAULT ACL; Schema: public; Owner: med_tracker_owner
--

ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public GRANT SELECT,INSERT,DELETE,UPDATE ON TABLES TO med_tracker_app;


--
-- PostgreSQL database dump complete
--



--
-- Version bookkeeping: schema_migrations and ar_internal_metadata rows so that
-- Rails and any future migrator treat this baseline as already applied.
--

--
-- PostgreSQL database dump
--


-- Dumped from database version 18.6
-- Dumped by pg_dump version 18.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Data for Name: ar_internal_metadata; Type: TABLE DATA; Schema: public; Owner: medtracker
--

COPY public.ar_internal_metadata (key, value, created_at, updated_at) FROM stdin;
environment	test	2026-10-04 22:10:10.367408	2026-10-04 22:10:10.367411
schema_sha1	3ecea2fa11c0bbb1bd514e8587c2043da32f231c	2026-10-04 22:10:10.371089	2026-10-04 22:10:10.37109
\.


--
-- Data for Name: schema_migrations; Type: TABLE DATA; Schema: public; Owner: medtracker
--

COPY public.schema_migrations (version) FROM stdin;
20261002174039
20260926000000
20260919131000
20260919130000
20260911123000
20260911122000
20260911121000
20260911120000
20260909020000
20260908200000
20260908120000
20260906190000
20260903140000
20260902100000
20260902090000
20260810203000
20260803150000
20260802071000
20260802070000
20260729120000
20260729103000
20260728170000
20260719090000
20260717130000
20260717120000
20260716120000
20260714090000
20260713210000
20260713190200
20260713190100
20260713190000
20260713150000
20260713130000
20260713123000
20260713090000
20260710210701
20260710130200
20260710130100
20260710130000
20260710120000
20260710110100
20260710110000
20260709150000
20260709143100
20260709143000
20260709131100
20260709131000
20260709130000
20260709123000
20260707110200
20260707110100
20260707110000
20260705120500
20260705120000
20260702172000
20260702170000
20260702160600
20260702154500
20260702143000
20260702133700
20260630183000
20260630141000
20260629120000
20260624091200
20260624091100
20260624091000
20260624090000
20260623090000
20260622001300
20260622001200
20260622001100
20260622001000
20260622000900
20260622000800
20260622000700
20260622000600
20260622000500
20260622000400
20260622000300
20260622000200
20260622000100
20260611120000
20260609210800
20260608090000
20260525090000
20260512160000
20260511120000
20260510120000
20260509000000
20260508130000
20260508120000
20260506123000
20260506120000
20260505120000
20260430090000
20260429120000
20260428100000
20260426000100
20260424120000
20260423120000
20260423110000
20260422120000
20260420110000
20260418083600
20260418083500
20260416161000
20260413152000
20260413143000
20260324090000
20260322000001
20260317115313
20260313000001
20260310213044
20260306133000
20260306120000
20260303113000
20260302110000
20260302100605
20260226155133
20260226155132
20260225230121
20260225144033
20260225105418
20260224160000
20260224155754
20260223231214
20260223221638
20260223221637
20260223221636
20260223221635
20260223221634
20260223173000
20260221233012
20260202074737
20260127134200
20260126162420
20260126161624
20260105160155
20251228210254
20251203164512
20251202112302
20251202093038
20251202091909
20251202083837
20251201214500
20251128215700
20251128215600
20251127231020
20251126133100
20251126111454
20251115121655
20251115121548
20251113221210
20251113101500
20251112154132
20251112140414
20251112140002
20251103230827
20251103225306
20251023221112
20250930220454
20250930220140
20250930215217
20250930214552
20250930214350
20250930214345
20250930162149
20250929163001
20250929081208
20250625132656
20250625131744
20250623132218
20250623132209
20250623132158
20250623132149
20250623132121
20250623132120
\.


--
-- PostgreSQL database dump complete
--


