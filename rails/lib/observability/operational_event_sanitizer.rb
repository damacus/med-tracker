# frozen_string_literal: true

module Observability
  module OperationalEventSanitizer
    SCALAR_VALIDATORS = {
      http_method: ->(value) { value.to_s.match?(/\A(?:DELETE|GET|HEAD|OPTIONS|PATCH|POST|PUT)\z/) },
      route: lambda { |value|
        value.is_a?(String) &&
          value.bytesize <= 256 &&
          value.start_with?('/') &&
          value.match?(%r{\A[A-Za-z0-9_/.:()\-*]+\z})
      },
      status_code: ->(value) { value.is_a?(Integer) && value.between?(100, 599) },
      duration: ->(value) { value.is_a?(Integer) && value >= 0 },
      job_class: ->(value) { value.to_s.match?(/\A[A-Z][A-Za-z0-9_:]{0,127}\z/) },
      job_queue: ->(value) { value.to_s.match?(/\A[a-z0-9_-]{1,64}\z/) },
      processed_count: ->(value) { value.is_a?(Integer) && value >= 0 },
      verified_count: ->(value) { value.is_a?(Integer) && value >= 0 },
      failed_count: ->(value) { value.is_a?(Integer) && value >= 0 }
    }.freeze

    private

    def event_options(options)
      {
        attributes: {},
        dataset: self.class::DATASET,
        service: nil,
        context: ContextEnricher.call,
        clock: -> { Time.current },
        event_id: SecureRandom.uuid,
        error_type: nil,
        error: nil,
        message: nil
      }.merge(options)
    end

    def safe_message(message)
      return if message.blank?

      self.class::MESSAGES[message.to_sym]
    end

    def safe_error_type(error_type)
      name = error_type.respond_to?(:name) ? error_type.name : error_type.to_s
      name if name.match?(/\A[A-Z][A-Za-z0-9_:]{0,127}\z/)
    end

    def safe_attributes(name, attributes)
      permitted_keys = self.class::EVENT_ATTRIBUTE_KEYS.fetch(name.to_sym)
      attributes.each_with_object({}) do |(key, value), safe|
        next unless permitted_keys.include?(key.to_sym)

        field = self.class::ATTRIBUTE_FIELDS[key.to_sym]
        safe[field] = normalize_scalar(key.to_sym, value) if field
      end.compact
    end

    def normalize_scalar(key, value)
      allowed_values = self.class::CATEGORICAL_VALUES[key]
      return value.to_s if allowed_values&.include?(value.to_s)
      return value if valid_scalar?(key, value)

      value if value.is_a?(Integer) || value.equal?(true) || value.equal?(false)
    end

    def valid_scalar?(key, value)
      SCALAR_VALIDATORS.fetch(key, ->(_) { false }).call(value)
    end

    def safe_context(context)
      fields = context.respond_to?(:to_event_fields) ? context.to_event_fields : context.to_h
      workflow_context(fields).merge(execution_context(fields)).merge(trace_context(fields)).compact
    end

    def workflow_context(fields)
      {
        'medtracker.workflow.id' => uuid(fields['medtracker.workflow.id']),
        'medtracker.causation.id' => uuid(fields['medtracker.causation.id']),
        'medtracker.attempt.id' => uuid(fields['medtracker.attempt.id'])
      }
    end

    def execution_context(fields)
      {
        'medtracker.request.id' => opaque_identifier(fields['medtracker.request.id']),
        'medtracker.job.id' => opaque_identifier(fields['medtracker.job.id'])
      }
    end

    def trace_context(fields)
      {
        'trace.id' => hex_identifier(fields['trace.id'], 32),
        'span.id' => hex_identifier(fields['span.id'], 16)
      }
    end

    def uuid(value)
      value if valid_uuid?(value)
    end

    def opaque_identifier(value)
      value if value.to_s.match?(/\A[A-Za-z0-9_.:-]{1,128}\z/)
    end

    def hex_identifier(value, length)
      value if value.to_s.match?(/\A[0-9a-f]{#{length}}\z/)
    end
  end
end
