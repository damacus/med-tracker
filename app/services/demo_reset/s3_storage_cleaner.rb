# frozen_string_literal: true

require 'aws-sdk-s3'

module DemoReset
  class S3StorageCleaner
    DELETE_BATCH_SIZE = 1000
    Target = Data.define(:endpoint, :bucket)

    def self.runtime_target
      Target.new(
        endpoint: ENV.fetch('ACTIVE_STORAGE_S3_ENDPOINT', nil),
        bucket: ENV.fetch('ACTIVE_STORAGE_S3_BUCKET', nil)
      )
    end

    def self.expected_target
      Target.new(
        endpoint: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_ENDPOINT', nil),
        bucket: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_BUCKET', nil)
      )
    end

    def initialize(client: nil, target: self.class.runtime_target, expected_target: self.class.expected_target)
      @target = target.is_a?(Target) ? target : Target.new(**target)
      @expected_target = expected_target.is_a?(Target) ? expected_target : Target.new(**expected_target)
      @client = client || Aws::S3::Client.new(client_options)
    end

    def call
      verify_target!
      keys = object_keys
      keys.each_slice(DELETE_BATCH_SIZE) { |batch| delete_objects(batch) }
      raise StorageCleanupError, 'storage cleanup failed' unless empty?

      { objects_removed: keys.size }
    rescue UnsafeTargetError, StorageCleanupError
      raise
    rescue Aws::S3::Errors::ServiceError
      raise StorageCleanupError, 'storage cleanup failed'
    end

    def empty?
      verify_target!
      object_keys(max_keys: 1).empty?
    rescue UnsafeTargetError, StorageCleanupError
      raise
    rescue Aws::S3::Errors::ServiceError
      raise StorageCleanupError, 'storage cleanup failed'
    end

    private

    attr_reader :client, :target, :expected_target

    def verify_target!
      unless target_matches?(target.endpoint, expected_target.endpoint)
        raise UnsafeTargetError, 'demo reset refused: storage_endpoint'
      end
      return if target_matches?(target.bucket, expected_target.bucket)

      raise UnsafeTargetError, 'demo reset refused: storage_bucket'
    end

    def target_matches?(actual, expected)
      expected.present? && actual == expected
    end

    def object_keys(max_keys: nil)
      keys = []
      continuation_token = nil
      loop do
        response = client.list_objects_v2(list_options(continuation_token:, max_keys:))
        keys.concat(response.contents.map(&:key))
        break unless response.is_truncated

        continuation_token = response.next_continuation_token
        raise StorageCleanupError, 'storage cleanup failed' if continuation_token.blank?
      end
      keys
    end

    def list_options(continuation_token:, max_keys:)
      { bucket: target.bucket, continuation_token:, max_keys: }.compact
    end

    def delete_objects(keys)
      response = client.delete_objects(
        bucket: target.bucket,
        delete: { objects: keys.map { |key| { key: } }, quiet: true }
      )
      raise StorageCleanupError, 'storage cleanup failed' if response.errors.any?
    end

    def client_options
      {
        endpoint: target.endpoint,
        region: ENV.fetch('ACTIVE_STORAGE_S3_REGION'),
        access_key_id: ENV.fetch('ACTIVE_STORAGE_S3_ACCESS_KEY_ID'),
        secret_access_key: ENV.fetch('ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY'),
        force_path_style: ENV.fetch('ACTIVE_STORAGE_S3_FORCE_PATH_STYLE', 'true') == 'true'
      }
    end
  end
end
