# frozen_string_literal: true

module HouseholdLifecycleTasks
  class Failure < StandardError
    attr_reader :event_type

    def initialize(event_type, failure_class)
      @event_type = event_type
      super("#{event_type} failed (#{failure_class})")
    end
  end

  class << self
    def execute(event_type)
      yield
    rescue StandardError => e
      warn JSON.generate(event_type: event_type, outcome: 'failure', failure_code: e.class.name)
      raise Failure.new(event_type, e.class.name), cause: nil
    end

    def household
      Household.find(required('HOUSEHOLD_ID'))
    end

    def actor_account
      Account.find(required('ACTOR_ACCOUNT_ID'))
    end

    def membership(household:, actor_account:)
      TenantContext.with(account: actor_account, household: household) do
        HouseholdMembership.find(required('MEMBERSHIP_ID'))
      end
    end

    def export(household:, actor_account:)
      TenantContext.with(account: actor_account, household: household) do
        HouseholdExport.find(required('EXPORT_ID'))
      end
    end

    def required(name)
      ENV.fetch(name).presence || raise(KeyError, "#{name} is required")
    end
  end
end

namespace :household_lifecycle do
  task export: :environment do
    HouseholdLifecycleTasks.execute('household.export.ready') do
      household = HouseholdLifecycleTasks.household
      actor_account = HouseholdLifecycleTasks.actor_account
      export = Households::HostedExport.generate!(
        household: household,
        membership: HouseholdLifecycleTasks.membership(household: household, actor_account: actor_account),
        actor_account: actor_account
      )
      puts JSON.generate(
        event_type: 'household.export.ready',
        outcome: export.status,
        household_id: household.id,
        export_id: export.id,
        attachment_count: export.manifest.fetch('attachments').size
      )
    end
  end

  task download: :environment do
    HouseholdLifecycleTasks.execute('household.export.downloaded') do
      household = HouseholdLifecycleTasks.household
      actor_account = HouseholdLifecycleTasks.actor_account
      export = HouseholdLifecycleTasks.export(household: household, actor_account: actor_account)
      result = Households::HostedExportTransfer.call(
        export: export,
        actor_account: actor_account,
        destination: HouseholdLifecycleTasks.required('DESTINATION')
      )
      puts JSON.generate(
        event_type: 'household.export.downloaded',
        outcome: 'downloaded',
        household_id: household.id,
        export_id: export.id,
        artifact_byte_size: result.artifact_byte_size,
        artifact_checksum_sha256: result.artifact_checksum_sha256
      )
    end
  end

  task hold: :environment do
    HouseholdLifecycleTasks.execute('household.retention_hold.placed') do
      household = HouseholdLifecycleTasks.household
      hold = Households::RetentionHoldManager.place!(
        household: household,
        actor_account: HouseholdLifecycleTasks.actor_account,
        reason: HouseholdLifecycleTasks.required('REASON'),
        review_on: Date.iso8601(HouseholdLifecycleTasks.required('REVIEW_ON'))
      )
      puts JSON.generate(
        event_type: 'household.retention_hold.placed',
        outcome: hold.status,
        household_id: household.id,
        retention_hold_id: hold.id,
        review_on: hold.review_on.iso8601
      )
    end
  end

  task release_hold: :environment do
    HouseholdLifecycleTasks.execute('household.retention_hold.released') do
      household = HouseholdLifecycleTasks.household
      actor_account = HouseholdLifecycleTasks.actor_account
      hold = TenantContext.with(account: actor_account, household: household) do
        HouseholdRetentionHold.find(HouseholdLifecycleTasks.required('HOLD_ID'))
      end
      Households::RetentionHoldManager.release!(hold: hold, actor_account: actor_account)
      puts JSON.generate(
        event_type: 'household.retention_hold.released',
        outcome: hold.status,
        household_id: household.id,
        retention_hold_id: hold.id
      )
    end
  end

  task offboard: :environment do
    HouseholdLifecycleTasks.execute('household.offboarded') do
      household = HouseholdLifecycleTasks.household
      Households::Offboarder.call(household: household, actor_account: HouseholdLifecycleTasks.actor_account)
      puts JSON.generate(event_type: 'household.offboarded', outcome: household.lifecycle_state,
                         household_id: household.id)
    end
  end

  task purge: :environment do
    HouseholdLifecycleTasks.execute('household.purge.completed') do
      household = HouseholdLifecycleTasks.household
      run = Households::Purger.call(household: household, actor_account: HouseholdLifecycleTasks.actor_account)
      puts JSON.generate(
        event_type: 'household.purge.completed',
        outcome: run.status,
        household_id: household.id,
        purge_run_id: run.id,
        attempts: run.attempts,
        last_completed_table: run.last_completed_table
      )
    end
  end
end
