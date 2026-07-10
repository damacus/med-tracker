# frozen_string_literal: true

require 'aws-sdk-s3'

module Audit
  module ObjectLock
    WriteResult = Data.define(:object_key, :checksum_sha256, :version_id, :retention_mode, :retain_until)

    class Error < StandardError; end
    class ConfigurationError < Error; end
    class RetryableError < Error; end
    class IntegrityError < Error; end

    class S3Adapter
      def initialize(configuration:, client: nil)
        @configuration = configuration
        @client = client || Aws::S3::Client.new(client_options)
      end

      def validate!
        client.head_bucket(bucket:, expected_bucket_owner:)
        validate_versioning!
        validate_object_lock!
        validate_encryption!
        true
      rescue Aws::S3::Errors::ServiceError => e
        raise ConfigurationError, "audit Object Lock bucket validation failed: #{e.class.name}"
      end

      def write(record)
        serializer = RecordSerializer.new(record)
        response = client.put_object(put_attributes(serializer))
        result(serializer, response.version_id)
      rescue Aws::S3::Errors::PreconditionFailed
        verify_existing(serializer)
      rescue Aws::S3::Errors::ServiceError => e
        raise RetryableError, "audit Object Lock write failed: #{e.class.name}"
      end

      def verify(record, delivery:)
        serializer = RecordSerializer.new(record)
        validate_delivery_receipt!(serializer, delivery)
        response = delivered_object_head(serializer, delivery)
        validate_delivered_object!(response, serializer, delivery)
        validate_single_version!(serializer)
        true
      rescue Aws::S3::Errors::NoSuchKey, Aws::S3::Errors::NotFound
        raise IntegrityError, 'audit object is missing'
      rescue Aws::S3::Errors::ServiceError => e
        raise RetryableError, "audit Object Lock verification failed: #{e.class.name}"
      end

      private

      attr_reader :configuration, :client

      def delivered_object_head(serializer, delivery)
        client.head_object(
          bucket:, key: serializer.object_key, version_id: delivery.object_version_id,
          expected_bucket_owner:, checksum_mode: 'ENABLED'
        )
      end

      def validate_delivered_object!(response, serializer, delivery)
        validate_existing_checksum!(response, serializer)
        validate_retention_until!(response, [serializer.retain_until, delivery.retain_until].compact.max)
        validate_retention_mode!(response, delivery.retention_mode)
        validate_object_version!(response, delivery)
      end

      def bucket
        configuration.bucket
      end

      def expected_bucket_owner
        configuration.expected_owner
      end

      def client_options
        {
          region: configuration.region, endpoint: configuration.endpoint,
          force_path_style: configuration.force_path_style?
        }.compact
      end

      def validate_versioning!
        response = client.get_bucket_versioning(bucket:, expected_bucket_owner:)
        raise ConfigurationError, 'audit bucket versioning must be Enabled' unless response.status == 'Enabled'
      end

      def validate_object_lock!
        response = client.get_object_lock_configuration(bucket:, expected_bucket_owner:)
        return if response.object_lock_configuration&.object_lock_enabled == 'Enabled'

        raise ConfigurationError, 'audit bucket Object Lock must be Enabled'
      end

      def validate_encryption!
        response = client.get_bucket_encryption(bucket:, expected_bucket_owner:)
        algorithms = response.server_side_encryption_configuration.rules.filter_map do |rule|
          rule.apply_server_side_encryption_by_default&.sse_algorithm
        end
        return if algorithms.include?(configuration.server_side_encryption)

        raise ConfigurationError, 'audit bucket encryption does not match the configured mode'
      end

      def put_attributes(serializer)
        {
          bucket:, key: serializer.object_key, body: serializer.body,
          expected_bucket_owner:, if_none_match: '*', checksum_algorithm: 'SHA256',
          checksum_sha256: serializer.checksum_sha256_base64,
          metadata: { 'content-sha256' => serializer.checksum_sha256 },
          server_side_encryption: configuration.server_side_encryption,
          ssekms_key_id: configuration.kms_key_id,
          object_lock_mode: configuration.retention_mode,
          object_lock_retain_until_date: serializer.retain_until
        }.compact
      end

      def verify_existing(serializer)
        response = client.head_object(
          bucket:, key: serializer.object_key, expected_bucket_owner:, checksum_mode: 'ENABLED'
        )
        validate_existing_checksum!(response, serializer)
        validate_existing_retention!(response, serializer)
        validate_existing_mode!(response)
        result(serializer, response.version_id)
      rescue Aws::S3::Errors::ServiceError => e
        raise RetryableError, "existing audit object could not be verified: #{e.class.name}"
      end

      def validate_existing_checksum!(response, serializer)
        return if checksum_matches?(response, serializer)

        raise IntegrityError, 'existing audit object checksum does not match'
      end

      def validate_existing_retention!(response, serializer)
        return if retention_covers?(response, serializer.retain_until)

        raise IntegrityError, 'existing audit object retention is too short'
      end

      def validate_existing_mode!(response)
        validate_retention_mode!(response, configuration.retention_mode)
      end

      def validate_retention_mode!(response, expected_mode)
        return if response.object_lock_mode == expected_mode

        raise IntegrityError, 'existing audit object retention mode does not match'
      end

      def checksum_matches?(response, serializer)
        response.checksum_sha256 == serializer.checksum_sha256_base64 ||
          response.metadata['content-sha256'] == serializer.checksum_sha256
      end

      def retention_covers?(response, retain_until)
        response.object_lock_retain_until_date && response.object_lock_retain_until_date >= retain_until
      end

      def validate_delivery_receipt!(serializer, delivery)
        return if matching_receipt?(serializer, delivery) && complete_receipt?(delivery)

        raise IntegrityError, 'audit delivery receipt does not match the expected object'
      end

      def matching_receipt?(serializer, delivery)
        delivery.object_key == serializer.object_key && delivery.checksum_sha256 == serializer.checksum_sha256
      end

      def complete_receipt?(delivery)
        delivery.object_version_id.present? && delivery.retention_mode.present? && delivery.retain_until.present?
      end

      def validate_retention_until!(response, retain_until)
        return if retention_covers?(response, retain_until)

        raise IntegrityError, 'existing audit object retention is too short'
      end

      def validate_object_version!(response, delivery)
        return if response.version_id == delivery.object_version_id

        raise IntegrityError, 'audit object version does not match its delivery receipt'
      end

      def validate_single_version!(serializer)
        response = client.list_object_versions(bucket:, prefix: serializer.object_key, expected_bucket_owner:)
        versions = response.versions.select { |version| version.key == serializer.object_key }
        return if versions.one?

        raise IntegrityError, 'audit object has missing or duplicate versions'
      end

      def result(serializer, version_id)
        WriteResult.new(
          object_key: serializer.object_key, checksum_sha256: serializer.checksum_sha256,
          version_id:, retention_mode: configuration.retention_mode, retain_until: serializer.retain_until
        )
      end
    end
  end
end
