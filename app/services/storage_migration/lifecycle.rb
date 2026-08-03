# frozen_string_literal: true

module StorageMigration
  class Lifecycle
    Result = Data.define(
      :run_id,
      :eligible,
      :applied,
      :reason,
      :processed_count,
      :verified_count,
      :failed_count
    )

    Direction = Data.define(
      :source,
      :destination,
      :source_mirror,
      :destination_mirror
    )

    DIRECTIONS = {
      %w[persistent s3] => Direction.new(
        source: 'persistent',
        destination: 's3',
        source_mirror: 'persistent_with_s3_mirror',
        destination_mirror: 's3_with_persistent_mirror'
      ),
      %w[s3 persistent] => Direction.new(
        source: 's3',
        destination: 'persistent',
        source_mirror: 's3_with_persistent_mirror',
        destination_mirror: 'persistent_with_s3_mirror'
      )
    }.freeze

    DEFAULT_ROLLBACK_WINDOW = 24.hours

    def initialize(run:, blob_scope: ActiveStorage::Blob.all, **options)
      @run = run
      @blob_scope = blob_scope
      @service_registry = options.fetch(:service_registry, ActiveStorage::Blob.services)
      @owner_role = options.fetch(:owner_role, method(:owner_role?))
      @clock = options.fetch(:clock, -> { Time.current })
      @after_transition = options.fetch(:after_transition, -> {})
    end

    def reconcile(mutations_quiesced:, mirror_queue_drained:, apply: false)
      authorize!
      gate = reconciliation_gate(mutations_quiesced:, mirror_queue_drained:)
      return result(reason: gate) if gate

      backfill = reconcile_blobs(apply:)
      return migration_result(backfill, reason: :destination_unverified) unless backfill.successful?
      return migration_result(backfill, eligible: true, reason: :reconciliation_ready) unless apply

      record_reconciliation(backfill)
      migration_result(backfill, eligible: true, applied: true, reason: :reconciled)
    end

    def cutover(apply: false, rollback_window: DEFAULT_ROLLBACK_WINDOW)
      authorize!
      gate = cutover_gate
      return result(reason: gate) if gate

      return result(eligible: true, reason: :cutover_ready) unless apply

      transition(
        expected_services: [direction.source, direction.source_mirror],
        target_service: direction.destination_mirror,
        attributes: { phase: :rollback_window, rollback_deadline: clock.call + rollback_window },
        success_reason: :cutover_complete
      )
    end

    def rollback(source_verified:, mirror_queue_drained:, apply: false)
      authorize!
      return result(reason: :rollback_window_required) unless run.rollback_window?
      return result(reason: :rollback_window_expired) if rollback_window_expired?
      return result(reason: :source_unverified) unless source_verified
      return result(reason: :mirror_queue_pending) unless mirror_queue_drained
      return result(eligible: true, reason: :rollback_ready) unless apply

      transition(
        expected_services: [direction.destination_mirror],
        target_service: direction.source_mirror,
        attributes: reset_for_backfill,
        success_reason: :rollback_complete
      )
    end

    def finalize(acceptance_verified:, recovery_verified:, final_reconciled:, apply: false)
      authorize!
      gate = finalization_gate(acceptance_verified:, recovery_verified:, final_reconciled:)
      return result(reason: gate) if gate
      return result(eligible: true, reason: :finalization_ready) unless apply

      timestamp = clock.call
      transition(
        expected_services: [direction.destination_mirror],
        target_service: direction.destination,
        attributes: run.finalization_attributes(timestamp),
        success_reason: :finalized
      )
    end

    def retirement_eligibility(live_source_dependency:)
      authorize!
      return result(reason: :finalization_required) unless run.finalized?
      return result(reason: :acceptance_required) unless run.acceptance_verified_at
      return result(reason: :recovery_required) unless run.recovery_verified_at
      return result(reason: :final_reconciliation_required) unless run.final_reconciled_at
      return result(reason: :live_source_dependency) if live_source_dependency

      result(eligible: true, reason: :source_retirement_eligible)
    end

    private

    attr_reader :run, :blob_scope, :service_registry, :owner_role, :clock, :after_transition

    def direction
      DIRECTIONS.fetch([run.source_service_name, run.destination_service_name])
    end

    def authorize!
      raise SecurityError, 'storage migration requires the owner database role' unless owner_role.call
    end

    def stable_set?
      run.stable_blob_count == blob_scope.count &&
        blob_scope.where.not(service_name: [direction.source, direction.source_mirror]).none?
    end

    def rollback_window_expired?
      run.rollback_deadline && clock.call >= run.rollback_deadline
    end

    def reconciliation_gate(mutations_quiesced:, mirror_queue_drained:)
      return :mutations_not_quiesced unless mutations_quiesced

      :mirror_queue_pending unless mirror_queue_drained
    end

    def reconcile_blobs(apply:)
      Backfill.new(
        source_service_name: direction.source,
        destination_service_name: direction.destination,
        blob_scope:,
        service_registry:,
        run_id: run.run_id,
        copy_missing: apply
      ).call
    end

    def record_reconciliation(backfill)
      run.update!(
        phase: :reconciled,
        processed_count: backfill.processed_count,
        verified_count: backfill.verified_count,
        failed_count: backfill.failed_count,
        stable_blob_count: blob_scope.count,
        reconciled_at: clock.call,
        mirror_queue_drained_at: clock.call
      )
    end

    def cutover_gate
      return :reconciliation_required unless run.reconciled?
      return :mirror_queue_pending unless run.mirror_queue_drained_at

      :stable_set_changed unless stable_set?
    end

    def finalization_gate(acceptance_verified:, recovery_verified:, final_reconciled:)
      return :rollback_window_required unless run.rollback_window?
      return :rollback_window_open unless rollback_window_expired?
      return :acceptance_required unless acceptance_verified
      return :recovery_required unless recovery_verified

      :final_reconciliation_required unless final_reconciled
    end

    def transition(expected_services:, target_service:, attributes:, success_reason:)
      applied = false
      ActiveStorage::Blob.transaction do
        records = blob_scope.lock.to_a
        valid_transition = records.all? { expected_services.include?(it.service_name) }
        raise ActiveRecord::Rollback unless valid_transition

        update_services(records, target_service)
        run.update!(attributes)
        after_transition.call
        applied = true
      end
      return result(reason: :transaction_rolled_back) unless applied

      result(eligible: true, applied: true, reason: success_reason)
    rescue StandardError
      result(reason: :transaction_rolled_back)
    end

    def update_services(records, target_service)
      records.each { it.update!(service_name: target_service) }
    end

    def reset_for_backfill
      {
        phase: :backfill,
        stable_blob_count: nil,
        reconciled_at: nil,
        mirror_queue_drained_at: nil,
        rollback_deadline: nil
      }
    end

    def migration_result(backfill, reason:, eligible: false, applied: false)
      result(
        eligible:,
        applied:,
        reason:,
        processed_count: backfill.processed_count,
        verified_count: backfill.verified_count,
        failed_count: backfill.failed_count
      )
    end

    def result(reason:, **)
      Result.new(run_id: run.run_id,
                 eligible: false,
                 applied: false,
                 reason:,
                 processed_count: run.processed_count,
                 verified_count: run.verified_count,
                 failed_count: run.failed_count, **)
    end

    def owner_role?
      ActiveRecord::Base.connection.select_value('SELECT current_user') == 'med_tracker_owner'
    end
  end
end
