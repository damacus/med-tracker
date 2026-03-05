# frozen_string_literal: true

require 'lograge'
require 'ecs_logging/logger'

# A ProxyLogger that translates Hash messages to keyword arguments
# so that EcsLogging::Logger merges them into the root of the JSON
# instead of wrapping them in a {"message": "{...}"} string.
class LogrageEcsProxyLogger
  def initialize(logger)
    @logger = logger
  end

  [:debug, :info, :warn, :error, :fatal, :unknown].each do |level|
    define_method(level) do |msg = nil, &block|
      if msg.is_a?(Hash)
        @logger.send(level, nil, **msg)
      else
        @logger.send(level, msg, &block)
      end
    end
  end
end

class LogrageEcsFormatter
  def call(data)
    # Map Lograge data to ECS fields
    payload = {
      'http.request.method' => data[:method],
      'url.path' => data[:path],
      'http.response.status_code' => data[:status]
    }
    
    payload['event.duration'] = (data[:duration].to_f * 1_000_000).to_i if data[:duration]
    payload['message'] = "#{data[:method]} #{data[:path]} #{data[:status]}" if data[:method] && data[:path] && data[:status]

    payload.merge(data.except(:method, :path, :status, :duration))
  end
end

Rails.application.configure do
  config.lograge.enabled = true

  # Use our custom Hash-returning formatter
  config.lograge.formatter = LogrageEcsFormatter.new

  # Wrap the main logger in our proxy just for Lograge
  config.lograge.logger = LogrageEcsProxyLogger.new(config.logger)

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
