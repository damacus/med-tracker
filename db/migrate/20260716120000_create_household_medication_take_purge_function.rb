# frozen_string_literal: true

class CreateHouseholdMedicationTakePurgeFunction < ActiveRecord::Migration[8.1]
  FUNCTION_SIGNATURE = 'med_tracker.purge_medication_takes(bigint)'

  def up
    execute <<~SQL
      CREATE OR REPLACE FUNCTION med_tracker.purge_medication_takes(p_household_id bigint)
      RETURNS bigint
      LANGUAGE plpgsql
      SECURITY DEFINER
      SET search_path = pg_catalog
      AS $$
      DECLARE
        v_account_setting text := pg_catalog.current_setting('med_tracker.current_account_id', true);
        v_household_setting text := pg_catalog.current_setting('med_tracker.current_household_id', true);
        v_deleted_rows bigint;
      BEGIN
        IF v_household_setting IS NULL
           OR v_household_setting !~ '^[0-9]+$'
           OR v_household_setting::bigint IS DISTINCT FROM p_household_id THEN
          RAISE EXCEPTION USING
            ERRCODE = 'MT104',
            MESSAGE = 'household purge tenant context does not match target';
        END IF;

        IF v_account_setting IS NULL
           OR v_account_setting !~ '^[0-9]+$'
           OR NOT EXISTS (
             SELECT 1
             FROM public.platform_admins
             WHERE account_id = v_account_setting::bigint
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
      $$;

      REVOKE ALL ON FUNCTION #{FUNCTION_SIGNATURE} FROM PUBLIC;
      DO $purge_role_grant$
      BEGIN
        IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app') THEN
          GRANT EXECUTE ON FUNCTION #{FUNCTION_SIGNATURE} TO med_tracker_app;
        END IF;
      END
      $purge_role_grant$;
    SQL
  end

  def down
    execute "DROP FUNCTION IF EXISTS #{FUNCTION_SIGNATURE};"
  end
end
