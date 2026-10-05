# frozen_string_literal: true

module Audit
  module ObjectLock
    class Configuration
      class Invalid < StandardError; end

      RETENTION_MODES = %w[GOVERNANCE COMPLIANCE].freeze
      ENCRYPTION_MODES = %w[AES256 aws:kms].freeze

      attr_reader :bucket, :region, :expected_owner, :retention_mode, :server_side_encryption,
                  :kms_key_id, :endpoint

      def initialize(environment = ENV)
        @environment = environment
        @bucket = required('AUDIT_WORM_BUCKET')
        @region = required('AUDIT_WORM_REGION')
        @expected_owner = required('AUDIT_WORM_EXPECTED_OWNER')
        @retention_mode = environment.fetch('AUDIT_WORM_RETENTION_MODE', 'GOVERNANCE').upcase
        @server_side_encryption = environment.fetch('AUDIT_WORM_SSE', 'aws:kms')
        @kms_key_id = environment['AUDIT_WORM_KMS_KEY_ID'].presence
        @endpoint = environment['AUDIT_WORM_ENDPOINT'].presence
        validate!
      end

      def force_path_style?
        ActiveModel::Type::Boolean.new.cast(@environment['AUDIT_WORM_FORCE_PATH_STYLE'])
      end

      def compliance_approved?
        ActiveModel::Type::Boolean.new.cast(@environment['AUDIT_WORM_COMPLIANCE_APPROVED'])
      end

      def to_h
        {
          bucket:, region:, expected_owner:, retention_mode:, server_side_encryption:,
          kms_key_id:, endpoint:, force_path_style: force_path_style?
        }.compact
      end

      private

      def required(name)
        @environment[name].presence || raise(Invalid, "#{name} is required")
      end

      def validate!
        raise Invalid, "unsupported retention mode: #{retention_mode}" unless RETENTION_MODES.include?(retention_mode)
        raise Invalid, 'COMPLIANCE retention requires records-governance approval' if compliance_without_approval?
        raise Invalid, "unsupported server-side encryption: #{server_side_encryption}" unless encryption_valid?
        raise Invalid, 'a KMS key is required for aws:kms encryption' if kms_key_missing?
      end

      def compliance_without_approval?
        retention_mode == 'COMPLIANCE' && !compliance_approved?
      end

      def encryption_valid?
        ENCRYPTION_MODES.include?(server_side_encryption)
      end

      def kms_key_missing?
        server_side_encryption == 'aws:kms' && kms_key_id.blank?
      end
    end
  end
end
