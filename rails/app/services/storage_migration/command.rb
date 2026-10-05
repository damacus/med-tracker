# frozen_string_literal: true

require 'json'

module StorageMigration
  class Command
    class InputError < StandardError; end

    ACTIONS = %w[
      start resume reconcile cutover_eligibility cutover rollback finalize retirement_eligibility
    ].freeze
    APPLY_CONFIRMATION = {
      %w[persistent s3] => 'persistent-to-s3',
      %w[s3 persistent] => 's3-to-persistent'
    }.freeze
    SOURCE_PHASE = {
      %w[persistent s3] => 'persistent_with_s3_mirror',
      %w[s3 persistent] => 's3_with_persistent_mirror'
    }.freeze
    DESTINATION_PHASE = {
      %w[persistent s3] => 's3_with_persistent_mirror',
      %w[s3 persistent] => 'persistent_with_s3_mirror'
    }.freeze

    def initialize(
      environment: ENV,
      output: $stdout,
      error: $stderr,
      owner_role: -> { ActiveRecord::Base.connection.select_value('SELECT current_user') == 'med_tracker_owner' }
    )
      @environment = environment
      @output = output
      @error = error
      @owner_role = owner_role
    end

    def call
      raise SecurityError unless owner_role.call

      action = fetch('STORAGE_MIGRATION_ACTION')
      raise InputError, 'invalid_action' unless ACTIONS.include?(action)

      payload, status = action == 'start' ? start : continue(action)
      write(output, payload.merge(outcome: status.zero? ? 'passed' : 'blocked', action:))
      status
    rescue SecurityError
      failure('owner_role_required', 3)
    rescue InputError, KeyError, ActiveRecord::RecordInvalid, ActiveRecord::RecordNotFound => e
      failure(input_failure_code(e), 2)
    end

    private

    attr_reader :environment, :output, :error, :owner_role

    def start
      source, destination = direction_inputs
      validate_phase!(SOURCE_PHASE.fetch([source, destination]))
      ensure_confirmation!(source, destination)
      run_id = SecureRandom.uuid
      run = if apply?
              StorageMigrationRun.create!(source_service_name: source, destination_service_name: destination,
                                          run_id:)
            end
      migration = backfill(source:, destination:, run_id:, copy_missing: apply?)
      run&.update!(migration_counts(migration))
      [migration_payload(migration, run_id:), migration.successful? ? 0 : 1]
    end

    def continue(action)
      run = StorageMigrationRun.find_by!(run_id: fetch('STORAGE_MIGRATION_RUN_ID'))
      source, destination = direction_inputs
      raise InputError, 'migration_direction_mismatch' unless run_direction(run) == [source, destination]

      validate_action_phase!(action, source, destination)
      ensure_confirmation!(source, destination) if mutating_action?(action)
      return resume(run) if action == 'resume'

      lifecycle_result(action, run)
    end

    def resume(run)
      migration = backfill(
        source: run.source_service_name,
        destination: run.destination_service_name,
        run_id: run.run_id,
        copy_missing: apply?
      )
      run.update!(migration_counts(migration)) if apply?
      [migration_payload(migration, run_id: run.run_id), migration.successful? ? 0 : 1]
    end

    def lifecycle_result(action, run)
      lifecycle = Lifecycle.new(run:, owner_role: -> { true })
      operation = send("run_#{action}", lifecycle)
      [lifecycle_payload(operation), operation.eligible ? 0 : 1]
    end

    def run_reconcile(lifecycle)
      lifecycle.reconcile(
        apply: apply?,
        mutations_quiesced: enabled?('STORAGE_MIGRATION_MUTATIONS_QUIESCED'),
        mirror_queue_drained: enabled?('STORAGE_MIGRATION_MIRROR_QUEUE_DRAINED')
      )
    end

    def run_cutover_eligibility(lifecycle)
      lifecycle.cutover
    end

    def run_cutover(lifecycle)
      lifecycle.cutover(apply: apply?)
    end

    def run_rollback(lifecycle)
      lifecycle.rollback(
        apply: apply?,
        source_verified: enabled?('STORAGE_MIGRATION_SOURCE_VERIFIED'),
        mirror_queue_drained: enabled?('STORAGE_MIGRATION_MIRROR_QUEUE_DRAINED')
      )
    end

    def run_finalize(lifecycle)
      lifecycle.finalize(
        apply: apply?,
        acceptance_verified: enabled?('STORAGE_MIGRATION_ACCEPTANCE_VERIFIED'),
        recovery_verified: enabled?('STORAGE_MIGRATION_RECOVERY_VERIFIED'),
        final_reconciled: enabled?('STORAGE_MIGRATION_FINAL_RECONCILED')
      )
    end

    def run_retirement_eligibility(lifecycle)
      lifecycle.retirement_eligibility(
        live_source_dependency: enabled?('STORAGE_MIGRATION_LIVE_SOURCE_DEPENDENCY')
      )
    end

    def backfill(source:, destination:, run_id:, copy_missing:)
      Backfill.new(
        source_service_name: source,
        destination_service_name: destination,
        run_id:,
        copy_missing:
      ).call
    end

    def direction_inputs
      direction = [fetch('STORAGE_MIGRATION_SOURCE'), fetch('STORAGE_MIGRATION_DESTINATION')]
      raise InputError, 'invalid_storage_direction' unless APPLY_CONFIRMATION.key?(direction)

      direction
    end

    def validate_action_phase!(action, source, destination)
      expected = if %w[rollback finalize].include?(action)
                   DESTINATION_PHASE.fetch([source, destination])
                 elsif action == 'retirement_eligibility'
                   destination
                 else
                   SOURCE_PHASE.fetch([source, destination])
                 end
      validate_phase!(expected)
    end

    def validate_phase!(expected)
      raise InputError, 'storage_phase_mismatch' unless fetch('STORAGE_MIGRATION_PHASE') == expected
    end

    def ensure_confirmation!(source, destination)
      return unless apply?
      return if environment['STORAGE_MIGRATION_CONFIRM'] == APPLY_CONFIRMATION.fetch([source, destination])

      raise InputError, 'confirmation_required'
    end

    def mutating_action?(action)
      apply? && %w[cutover_eligibility retirement_eligibility].exclude?(action)
    end

    def apply?
      enabled?('STORAGE_MIGRATION_APPLY')
    end

    def enabled?(key)
      environment[key] == 'true'
    end

    def fetch(key)
      environment.fetch(key).presence || raise(KeyError, key)
    end

    def run_direction(run)
      [run.source_service_name, run.destination_service_name]
    end

    def migration_counts(migration)
      migration.to_h.slice(:processed_count, :verified_count, :failed_count)
    end

    def migration_payload(migration, run_id:)
      migration_counts(migration).merge(run_id:, applied: apply?,
                                        reason: migration.successful? ? 'completed' : 'failed')
    end

    def lifecycle_payload(operation)
      operation.to_h.slice(
        :run_id,
        :eligible,
        :applied,
        :reason,
        :processed_count,
        :verified_count,
        :failed_count
      )
    end

    def input_failure_code(error)
      return error.message if error.is_a?(InputError)
      return 'migration_run_not_found' if error.is_a?(ActiveRecord::RecordNotFound)
      return 'invalid_migration_state' if error.is_a?(ActiveRecord::RecordInvalid)

      'required_input_missing'
    end

    def failure(code, status)
      write(error, outcome: 'failed', failure_code: code)
      status
    end

    def write(io, payload)
      io.puts(JSON.generate(payload))
    end
  end
end
