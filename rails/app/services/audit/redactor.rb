# frozen_string_literal: true

module Audit
  class Redactor
    FILTERED = '[FILTERED]'
    SENSITIVE_KEYS = %w[
      access_token auth authorization bearer cookie endpoint id_token p256dh password password_digest password_hash
      public_key refresh_token secret session_id token token_digest webauthn_id
    ].freeze

    class << self
      def call(value)
        case value
        when Hash
          value.each_with_object({}) do |(key, nested_value), redacted|
            redacted[key] = sensitive?(key) ? FILTERED : call(nested_value)
          end
        when Array
          value.map { |nested_value| call(nested_value) }
        else
          value
        end
      end

      private

      def sensitive?(key)
        SENSITIVE_KEYS.include?(key.to_s.downcase)
      end
    end
  end
end
