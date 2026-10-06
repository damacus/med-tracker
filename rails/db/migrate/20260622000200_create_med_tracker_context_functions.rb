# frozen_string_literal: true

class CreateMedTrackerContextFunctions < ActiveRecord::Migration[8.1]
  def up
    execute <<~SQL
      CREATE SCHEMA IF NOT EXISTS med_tracker;

      CREATE OR REPLACE FUNCTION med_tracker.current_account_id()
      RETURNS bigint
      LANGUAGE sql
      STABLE
      AS $$
        SELECT NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint;
      $$;

      CREATE OR REPLACE FUNCTION med_tracker.current_household_id()
      RETURNS bigint
      LANGUAGE sql
      STABLE
      AS $$
        SELECT NULLIF(current_setting('med_tracker.current_household_id', true), '')::bigint;
      $$;

      CREATE OR REPLACE FUNCTION med_tracker.current_membership_id()
      RETURNS bigint
      LANGUAGE sql
      STABLE
      AS $$
        SELECT NULLIF(current_setting('med_tracker.current_membership_id', true), '')::bigint;
      $$;
    SQL
  end

  def down
    execute <<~SQL
      DROP FUNCTION IF EXISTS med_tracker.current_membership_id();
      DROP FUNCTION IF EXISTS med_tracker.current_household_id();
      DROP FUNCTION IF EXISTS med_tracker.current_account_id();
      DROP SCHEMA IF EXISTS med_tracker;
    SQL
  end
end
