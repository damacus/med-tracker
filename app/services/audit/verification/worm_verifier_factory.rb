# frozen_string_literal: true

module Audit
  module Verification
    class WormVerifierFactory
      def initialize(environment = ENV)
        @environment = environment
      end

      def call
        WormVerifier.new(records: selected_records, adapter: validated_adapter)
      rescue Audit::EntryFilter::Invalid, Audit::ObjectLock::Configuration::Invalid => e
        raise ConfigurationError, e.message
      end

      private

      attr_reader :environment

      def selected_records
        entries + signed_checkpoints
      end

      def entries
        @entries ||= Audit::EntryFilter.new(environment).call.to_a
      end

      def signed_checkpoints
        keys = entries.to_h { |entry| [[entry.chain_key, entry.chain_epoch, entry.sequence], true] }
        AuditCheckpoint.where.not(signature: nil).select do |checkpoint|
          keys.key?([checkpoint.chain_key, checkpoint.chain_epoch, checkpoint.sequence])
        end
      end

      def validated_adapter
        configuration = Audit::ObjectLock::Configuration.new(environment)
        Audit::ObjectLock::S3Adapter.new(configuration:).tap(&:validate!)
      end
    end
  end
end
