# frozen_string_literal: true

module Otel
  class SpanSanitizer
    EMAIL_PATTERN = /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/
    DATE_ONLY_PATTERN = /\b\d{4}-\d{2}-\d{2}\b(?!T)/
    DATE_DMY_PATTERN = %r{\b\d{2}/\d{2}/\d{4}\b}
    IP_PATTERN = /\b(?:\d{1,3}\.){3}\d{1,3}\b/

    SENSITIVE_KEY_PATTERNS = [
      /authorization/i,
      /cookie/i,
      /password/i,
      /secret/i,
      /token/i,
      /\bdate_of_birth\b/i,
      /\bdob\b/i
    ].freeze

    PII_KEY_PATTERNS = [
      /(?<!\bmodel\.)(?:^|\.)name$/i,
      /(?<!\bmodel\.)(?:^|\.)email/i
    ].freeze

    def sanitize_value(value)
      return value unless value.is_a?(String)

      result = value.dup
      result = result.gsub(EMAIL_PATTERN, '[EMAIL REDACTED]')
      result = result.gsub(IP_PATTERN, '[IP REDACTED]')
      redact_date_only(result)
    end

    def sanitize_attributes(attrs)
      attrs.each_with_object({}) do |(key, value), sanitized|
        sanitized[key] = if self.class.sensitive_key?(key)
                           '[REDACTED]'
                         else
                           sanitize_value(value)
                         end
      end
    end

    def self.sensitive_key?(key)
      return false if key == 'model.name'
      return false if key == 'model.operation'
      return false if key == 'model.id'

      SENSITIVE_KEY_PATTERNS.any? { |pattern| key.match?(pattern) } ||
        PII_KEY_PATTERNS.any? { |pattern| key.match?(pattern) }
    end

    private

    def redact_date_only(value)
      result = value.gsub(DATE_DMY_PATTERN, '[DATE REDACTED]')
      result.gsub(DATE_ONLY_PATTERN, '[DATE REDACTED]')
    end
  end
end
