# frozen_string_literal: true

module Observability
  module EventMappingSupport
    private

    def event(name, outcome, severity, reason, attributes)
      { name:, outcome:, severity:, reason:, attributes: attributes.compact }
    end

    def source_attributes(payload)
      { source_category: EventMapper::SOURCE_CATEGORIES[payload[:source_type].to_s] }.compact
    end

    def bounded_integer(value)
      Integer(value).clamp(0, 2_147_483_647)
    rescue ArgumentError, TypeError
      0
    end
  end
end
