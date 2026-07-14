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
          preserve_shared_login_identities!(household, actor_account) if table_name == 'people'
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
        ApiSession.where(household_membership_id: membership_ids).delete_all
        ApiAppToken.where(household_membership_id: membership_ids).delete_all
        OauthGrant.where(household_membership_id: membership_ids).delete_all
      end

      def preserve_shared_login_identities!(household, actor_account)
        people = TenantContext.with(account: actor_account, household: household) do
          Person.where(household: household).includes(:account, :user).to_a
        end

        people.each do |person|
          next if person.user.blank?

          preserve_or_remove_login_identity!(person, household)
        end
      end

      def preserve_or_remove_login_identity!(person, household)
        membership = surviving_membership(person.account, household)
        return User.where(id: person.user.id).delete_all if membership.blank?

        preserve_login_identity!(person, membership)
      end

      def surviving_membership(account, household)
        return if account.blank?

        TenantContext.with(account: account, household: household) do
          HouseholdMembership.active.joins(:household).merge(Household.operational)
                             .where(account: account).where.not(household: household)
                             .includes(:household, :person).order(:id).first
        end
      end

      def preserve_login_identity!(person, membership)
        TenantContext.with(account: person.account, household: membership.household, membership: membership) do
          identity = reusable_identity(person.account, membership.household) ||
                     build_identity(person, membership.household)
          identity.save!(validate: false) if identity.new_record?
          person.user.update!(person: identity)
          membership.update!(person: identity) if membership.person.blank?
        end
      end

      def reusable_identity(account, household)
        Person.where(household: household, account: account).left_outer_joins(:user).find_by(users: { id: nil })
      end

      def build_identity(person, household)
        Person.new(
          account: person.account,
          household: household,
          name: person.name,
          date_of_birth: person.date_of_birth,
          person_type: person.person_type,
          has_capacity: person.has_capacity,
          professional_title: person.professional_title
        )
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
