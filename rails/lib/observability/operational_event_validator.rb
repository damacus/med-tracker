# frozen_string_literal: true

module Observability
  module OperationalEventValidator
    private

    def validate!(options)
      validation_rules(options).each do |valid, message|
        raise ArgumentError, message unless valid
      end
    end

    def validation_rules(options)
      [
        [registered_name?(options), 'unregistered event name'],
        [registered_outcome?(options), 'invalid event outcome'],
        [registered_severity?(options), 'invalid log severity'],
        [registered_reason?(options), 'unregistered event reason'],
        [valid_uuid?(options.fetch(:event_id)), 'invalid event identifier'],
        [registered_dataset?(options), 'invalid event dataset']
      ]
    end

    def registered_name?(options)
      self.class::EVENT_NAMES.key?(options.fetch(:name).to_sym)
    end

    def registered_outcome?(options)
      self.class::OUTCOMES.include?(options.fetch(:outcome).to_sym)
    end

    def registered_severity?(options)
      self.class::SEVERITIES.include?(options.fetch(:severity).to_sym)
    end

    def registered_reason?(options)
      self.class::REASONS.include?(options.fetch(:reason).to_sym)
    end

    def registered_dataset?(options)
      self.class::DATASETS.include?(options.fetch(:dataset))
    end

    def valid_uuid?(value)
      value.to_s.match?(CorrelationContext::UUID_PATTERN)
    end
  end
end
