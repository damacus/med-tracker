# frozen_string_literal: true

module Observability
  module StorageEvent
    BACKENDS = {
      persistent: :disk,
      persistent_with_s3_mirror: :dual,
      s3_with_persistent_mirror: :dual,
      s3: :s3
    }.freeze

    module_function

    def configuration(service:)
      emit(
        service:,
        event: {
          operation: :configuration,
          outcome: :success,
          reason: :configured,
          counts: { processed_count: 0, verified_count: 0, failed_count: 0 }
        }
      )
    end

    def migration(service:, outcome:, reason:, counts:, error: nil)
      emit(
        service:,
        event: { operation: :migration, outcome:, reason:, counts:, error: }
      )
    end

    def emit(service:, event:)
      phase = service.to_sym
      Publisher.emit(
        name: :storage_stage,
        outcome: event.fetch(:outcome),
        severity: event.fetch(:outcome).to_sym == :failure ? :error : :info,
        reason: event.fetch(:reason),
        attributes: {
          storage_operation: event.fetch(:operation),
          storage_backend: BACKENDS.fetch(phase),
          storage_phase: phase,
          **event.fetch(:counts)
        },
        error_type: event[:error]&.class
      )
    end
    private_class_method :emit
  end
end
