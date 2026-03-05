# frozen_string_literal: true

require 'lograge'
require 'ecs_logging/logger'

Rails.application.configure do
  config.lograge.enabled = true

  # Use JSON formatter for Lograge (ECS logging is handled by the main logger)
  config.lograge.formatter = Lograge::Formatters::Json.new

  # Keep standard Rails logs for other things, but Lograge handles requests
  config.lograge.keep_appenders = false

  # Add custom data to Lograge entries, including OpenTelemetry correlation
  config.lograge.custom_options = lambda do |event|
    options = {}

    # OpenTelemetry correlation
    current_span = OpenTelemetry::Trace.current_span
    if current_span.context.valid?
      options['trace.id'] = current_span.context.hex_trace_id
      options['span.id'] = current_span.context.hex_span_id
    end

    options
  end
end
