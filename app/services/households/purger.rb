# frozen_string_literal: true

module Households
  class Purger
    extend OperatorAuthorization

    class ActiveRetentionHold < StandardError
      def initialize
        super('Household has an active retention hold')
      end
    end

    PURGE_ORDER = %w[
      health_event_medications
      notification_events
      medication_review_prompts
      api_change_events
      api_idempotency_keys
      api_tombstones
      medication_takes
      schedules
      person_medications
      dosages
      notification_preferences
      location_memberships
      person_access_grants
      household_invitation_grants
      household_invitations
      carer_relationships
      health_events
      household_exports
      household_retention_holds
      medications
      locations
    ].freeze
    LOCK_NAMESPACE = 'med_tracker.household_purge'

    class << self
      def call(household:, actor_account:, after_table: nil)
        authorize_operator!(actor_account)
        with_purge_lock(household) do
          refuse_active_hold!(household, actor_account)
          Offboarder.call(household: household, actor_account: actor_account)
          run = purge_run(household, actor_account)
          next run if run.completed?

          execute_purge!(run, household, actor_account, after_table)
          run
        end
      end

      private

      def refuse_active_hold!(household, actor_account)
        active_hold = TenantContext.with(account: actor_account, household: household) do
          household.household_retention_holds.active.exists?
        end
        raise ActiveRetentionHold if active_hold
      end

      def purge_run(household, actor_account)
        HouseholdPurgeRun.acquire!(household: household, requested_by_account: actor_account)
      end

      def with_purge_lock(household)
        connection = ActiveRecord::Base.connection
        lock_key = "#{LOCK_NAMESPACE}:#{household.id}"
        acquire_purge_lock(connection, lock_key)
        yield
      ensure
        release_purge_lock(connection, lock_key) if connection && lock_key
      end

      def acquire_purge_lock(connection, lock_key)
        connection.select_value(
          ActiveRecord::Base.sanitize_sql_array(
            [
              'WITH lock_acquired AS MATERIALIZED (' \
              'SELECT pg_advisory_lock(hashtextextended(?, 0))' \
              ') SELECT 1 FROM lock_acquired',
              lock_key
            ]
          )
        )
      end

      def release_purge_lock(connection, lock_key)
        connection.select_value(
          ActiveRecord::Base.sanitize_sql_array(
            ['SELECT pg_advisory_unlock(hashtextextended(?, 0))', lock_key]
          )
        )
      end

      def start_run!(run)
        run.update!(
          status: :running,
          attempts: run.attempts + 1,
          started_at: run.started_at || Time.current,
          failure_code: nil,
          failed_at: nil
        )
      end

      def execute_purge!(run, household, actor_account, after_table)
        start_run!(run)
        prepare_purge!(household, actor_account)
        purge_dependencies!(run, household, actor_account, after_table)
        purge_inventory!(run, household, actor_account, after_table)
        complete_run!(run, household, actor_account)
      rescue StandardError => e
        run.reload.update!(status: :failed, failure_code: e.class.name, failed_at: Time.current)
        raise
      end

      def purge_dependencies!(run, household, actor_account, after_table)
        run_step!(run, household, actor_account, 'global_dependencies', after_table) do
          purge_global_dependencies(household)
        end
        run_step!(run, household, actor_account, 'active_storage_attachments', after_table) do
          purge_attachments(household)
        end
      end

      def purge_inventory!(run, household, actor_account, after_table)
        (PURGE_ORDER + %w[household_memberships people]).each do |table_name|
          run_step!(run, household, actor_account, table_name, after_table) do
            delete_household_rows(table_name, household.id)
          end
        end
      end

      def prepare_purge!(household, actor_account)
        TenantContext.with(account: actor_account, household: household) do
          household.lock!
          raise ActiveRetentionHold if household.household_retention_holds.active.exists?

          household.update!(lifecycle_state: :purging) unless household.purging?
        end
      end

      def run_step!(run, household, actor_account, table_name, after_table)
        TenantContext.with(account: actor_account, household: household) do
          yield
          run.update!(last_completed_table: table_name)
        end
        after_table&.call(table_name)
      end

      def complete_run!(run, household, actor_account)
        TenantContext.with(account: actor_account, household: household) do
          ActiveRecord::Base.transaction do
            ensure_inventory_empty!(household.id)
            household.update!(lifecycle_state: :purged)
            run.update!(status: :completed, completed_at: Time.current, failure_code: nil, failed_at: nil)
            record_completion(run, household, actor_account)
          end
        end
      end

      def purge_global_dependencies(household)
        membership_ids = HouseholdMembership.where(household: household).pluck(:id)
        person_ids = Person.where(household: household).pluck(:id)
        ApiSession.where(household_membership_id: membership_ids).delete_all
        ApiAppToken.where(household_membership_id: membership_ids).delete_all
        OauthGrant.where(household_membership_id: membership_ids).delete_all
        User.where(person_id: person_ids).delete_all
      end

      def purge_attachments(household)
        ActiveStorage::Attachment.where(household: household).includes(:blob).find_each do |attachment|
          blob = attachment.blob
          attachment.destroy!
          blob.destroy! unless ActiveStorage::Attachment.exists?(blob_id: blob.id)
        end
      end

      def delete_household_rows(table_name, household_id)
        connection = ActiveRecord::Base.connection
        connection.exec_delete(
          "DELETE FROM #{connection.quote_table_name(table_name)} " \
          "WHERE household_id = #{connection.quote(household_id)}"
        )
      end

      def record_completion(run, household, actor_account)
        Audit::Event.record!(
          household: household,
          actor_account: actor_account,
          event_type: 'household.purge.completed',
          metadata: {
            attempts: run.attempts,
            household_id: household.id,
            last_completed_table: run.last_completed_table,
            outcome: 'success',
            purge_run_id: run.id
          }
        )
      end

      def ensure_inventory_empty!(household_id)
        remaining = SchemaInventory.purgeable_household_owned_tables.select do |table_name|
          household_row_count(table_name, household_id).positive?
        end
        return if remaining.empty?

        raise ActiveRecord::ActiveRecordError, "Household purge incomplete for tables: #{remaining.join(', ')}"
      end

      def household_row_count(table_name, household_id)
        connection = ActiveRecord::Base.connection
        connection.select_value(
          "SELECT COUNT(*) FROM #{connection.quote_table_name(table_name)} " \
          "WHERE household_id = #{connection.quote(household_id)}"
        ).to_i
      end
    end
  end
end
