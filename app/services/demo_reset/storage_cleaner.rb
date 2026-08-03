# frozen_string_literal: true

require 'aws-sdk-s3'

module DemoReset
  class StorageCleaner
    DELETE_BATCH_SIZE = 1000

    def initialize(client: nil, service: ENV.fetch('ACTIVE_STORAGE_SERVICE', nil),
                   endpoint: ENV.fetch('ACTIVE_STORAGE_S3_ENDPOINT', nil),
                   bucket: ENV.fetch('ACTIVE_STORAGE_S3_BUCKET', nil))
      @service = service
      @endpoint = endpoint
      @bucket = bucket
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

    attr_reader :client, :service, :endpoint, :bucket

    def verify_target!
      raise UnsafeTargetError, 'demo reset refused: storage_service' unless service == Preflight::STORAGE_SERVICE
      raise UnsafeTargetError, 'demo reset refused: storage_endpoint' unless endpoint == Preflight::STORAGE_ENDPOINT
      raise UnsafeTargetError, 'demo reset refused: storage_bucket' unless bucket == Preflight::STORAGE_BUCKET
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
      { bucket:, continuation_token:, max_keys: }.compact
    end

    def delete_objects(keys)
      response = client.delete_objects(
        bucket:,
        delete: { objects: keys.map { |key| { key: } }, quiet: true }
      )
      raise StorageCleanupError, 'storage cleanup failed' if response.errors.any?
    end

    def client_options
      {
        endpoint:,
        region: ENV.fetch('ACTIVE_STORAGE_S3_REGION'),
        access_key_id: ENV.fetch('ACTIVE_STORAGE_S3_ACCESS_KEY_ID'),
        secret_access_key: ENV.fetch('ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY'),
        force_path_style: ENV.fetch('ACTIVE_STORAGE_S3_FORCE_PATH_STYLE', 'true') == 'true'
      }
    end
  end
end
