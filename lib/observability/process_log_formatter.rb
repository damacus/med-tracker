# frozen_string_literal: true

require 'json'
require 'time'

module Observability
  module ProcessLogFormatter
    DATASETS = %w[medtracker.puma medtracker.solid_queue medtracker.opentelemetry].freeze

    module_function

    def call(message, dataset:, severity: nil, time: Time.zone.now)
      raise ArgumentError, 'invalid process dataset' unless DATASETS.include?(dataset)

      begin
        format(message, dataset:, severity:, time:)
      rescue StandardError
        minimal_format(dataset:, severity:)
      end
    end

    def format(message, dataset:, severity:, time:)
      log_level = normalized_severity(message, severity)
      {
        '@timestamp' => time.utc.iso8601(3),
        'log.level' => log_level,
        'service.name' => 'medtracker',
        'service.component' => dataset.delete_prefix('medtracker.'),
        'event.name' => 'process.message',
        'event.dataset' => dataset,
        'process.pid' => Process.pid
      }.merge(opentelemetry_diagnostic(dataset:, log_level:)).to_json
    end
    private_class_method :format

    def minimal_format(dataset:, severity:)
      log_level = severity.to_s.downcase.presence || 'error'
      {
        '@timestamp' => Time.now.utc.iso8601(3),
        'log.level' => log_level,
        'service.name' => 'medtracker',
        'service.component' => dataset.delete_prefix('medtracker.'),
        'event.name' => 'process.message',
        'event.dataset' => dataset,
        'process.pid' => Process.pid
      }.merge(opentelemetry_diagnostic(dataset:, log_level:)).to_json
    end
    private_class_method :minimal_format

    def opentelemetry_diagnostic(dataset:, log_level:)
      return {} unless dataset == 'medtracker.opentelemetry' && log_level == 'error'

      { 'event.reason' => 'export_failed' }
    end
    private_class_method :opentelemetry_diagnostic

    def normalized_severity(message, severity)
      return severity.to_s.downcase if severity
      return 'error' if message.to_s.match?(/\b(?:ERROR|FATAL)\b/i)
      return 'warn' if message.to_s.match?(/\bWARN(?:ING)?\b/i)

      'info'
    end
    private_class_method :normalized_severity
  end
end
