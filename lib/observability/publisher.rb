# frozen_string_literal: true

module Observability
  module Publisher
    EXECUTION_STATE_KEY = :medtracker_observability_publishing

    module_function

    def emit(name:, **options)
      mapped_options = if EventMapper.event_names.include?(name.to_s)
                         EventMapper.map(name.to_s,
                                         options)
                       else
                         options.merge(name:)
                       end
      publish(OperationalEvent.build(**mapped_options))
    rescue StandardError => e
      emergency(error: e, event_name: name)
      nil
    end

    def publish(event)
      return if ActiveSupport::IsolatedExecutionState[EXECUTION_STATE_KEY]

      ActiveSupport::IsolatedExecutionState[EXECUTION_STATE_KEY] = true
      begin
        CanonicalLogger.write(event)
        event
      rescue StandardError => e
        emergency(error: e, event_name: event.to_h['event.name'])
        nil
      ensure
        ActiveSupport::IsolatedExecutionState.delete(EXECUTION_STATE_KEY)
      end
    end

    def emergency(error:, event_name:)
      EmergencyDiagnostic.write(error:, event_name:)
    rescue StandardError
      nil
    end
    private_class_method :emergency
  end
end
