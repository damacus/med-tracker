# frozen_string_literal: true

require 'json'
require 'time'

module Observability
  module ProcessLogFormatter
    DATASETS = %w[medtracker.puma medtracker.solid_queue medtracker.opentelemetry].freeze

    module_function

    def call(message, dataset:, severity: nil, time: Time.zone.now)
      raise ArgumentError, 'invalid process dataset' unless DATASETS.include?(dataset)

      {
        '@timestamp' => time.utc.iso8601(3),
        'log.level' => normalized_severity(message, severity),
        'service.name' => 'medtracker',
        'service.component' => dataset.delete_prefix('medtracker.'),
        'event.name' => 'process.message',
        'event.dataset' => dataset,
        'process.pid' => Process.pid
      }.to_json
    end

    def normalized_severity(message, severity)
      return severity.to_s.downcase if severity
      return 'error' if message.to_s.match?(/\b(?:ERROR|FATAL)\b/i)
      return 'warn' if message.to_s.match?(/\bWARN(?:ING)?\b/i)

      'info'
    end
    private_class_method :normalized_severity
  end
end
