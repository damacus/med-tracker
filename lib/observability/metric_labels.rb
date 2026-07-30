# frozen_string_literal: true

module Observability
  module MetricLabels
    ALLOWED_VALUES = {
      outcome: %w[success failure unknown],
      source_category: %w[schedule person_medication unknown]
    }.freeze

    module_function

    def build(**labels)
      ALLOWED_VALUES.each_with_object({}) do |(key, values), safe|
        value = labels[key].to_s
        safe[key.to_s] = value if values.include?(value)
      end
    end
  end
end
