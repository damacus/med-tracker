# frozen_string_literal: true

module Observability
  module DiagnosticEvent
    SUCCESS_REASONS = %i[configured recovered].freeze

    module_function

    def failure(component:, error:, severity: :error)
      emit(component:, reason: :operation_failed, severity:, error:)
    end

    def emit(component:, reason:, severity:, error: nil, **)
      Publisher.emit(
        name: :diagnostic,
        outcome: SUCCESS_REASONS.include?(reason.to_sym) ? :success : :failure,
        severity:,
        reason:,
        attributes: { diagnostic_component: component },
        error_type: error&.class
      )
    end
  end
end
