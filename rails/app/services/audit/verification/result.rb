# frozen_string_literal: true

module Audit
  module Verification
    class ConfigurationError < StandardError; end

    Issue = Data.define(:code, :message, :chain_key, :sequence, :metadata) do
      def initialize(code:, message:, chain_key:, sequence:, metadata: nil)
        super
      end

      def to_h
        { code:, message:, chain_key:, sequence:, metadata: }.compact
      end
    end

    Result = Data.define(:scope, :checked_entries, :checked_checkpoints, :checked_objects, :issues) do
      def valid?
        issues.empty?
      end

      def exit_code
        valid? ? 0 : 1
      end

      def issue_codes
        issues.map(&:code)
      end

      def to_h
        {
          scope:, valid: valid?, exit_code:, checked_entries:, checked_checkpoints:,
          checked_objects:, issues: issues.map(&:to_h)
        }
      end
    end
  end
end
