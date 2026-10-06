# frozen_string_literal: true

module Observability
  module MedicationTransactionOutcome
    module_function

    def register(dose_payload:, source_category:, context:)
      transaction = ActiveRecord::Base.current_transaction
      if transaction.open?
        transaction.after_commit { committed(dose_payload:, context:) }
        transaction.after_rollback { rolled_back(source_category:, context:) }
      else
        committed(dose_payload:, context:)
      end
    rescue StandardError => e
      Publisher.send(:emergency, error: e, event_name: 'medication_take.transaction_outcome')
      nil
    end

    def committed(dose_payload:, context:)
      with_context(context) do
        DomainEventPublisher.instrument('dose_taken.med_tracker', dose_payload)
      end
    end
    private_class_method :committed

    def rolled_back(source_category:, context:)
      with_context(context) do
        Publisher.emit(
          name: :medication_take_rolled_back,
          outcome: :failure,
          severity: :warn,
          reason: :rolled_back,
          attributes: { source_category: }
        )
      end
    end
    private_class_method :rolled_back

    def with_context(context)
      previous = Current.observability_context
      Current.observability_context = context
      yield
    ensure
      Current.observability_context = previous
    end
    private_class_method :with_context
  end
end
