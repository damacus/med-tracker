# frozen_string_literal: true

module StorageMigration
  class Backfill
    Result = Data.define(
      :run_id,
      :source_service_name,
      :destination_service_name,
      :phase,
      :processed_count,
      :verified_count,
      :failed_count
    ) do
      def successful?
        failed_count.zero?
      end
    end

    DIRECTIONS = {
      %i[persistent s3] => :persistent_with_s3_mirror,
      %i[s3 persistent] => :s3_with_persistent_mirror
    }.freeze
    DEFAULT_BATCH_SIZE = 100

    def initialize(source_service_name:, destination_service_name:, **options)
      @source_service_name = source_service_name.to_sym
      @destination_service_name = destination_service_name.to_sym
      @blob_scope = options.fetch(:blob_scope, ActiveStorage::Blob.all)
      @service_registry = options.fetch(:service_registry, ActiveStorage::Blob.services)
      @batch_size = Integer(options.fetch(:batch_size, DEFAULT_BATCH_SIZE))
      @run_id = options.fetch(:run_id, SecureRandom.uuid)
      @copy_missing = options.fetch(:copy_missing, true)
      validate!
    end

    def call
      counts = { processed_count: 0, verified_count: 0, failed_count: 0 }
      blob_scope.find_in_batches(batch_size:) do |blobs|
        blobs.each { process(it, counts) }
      end
      result = Result.new(
        run_id:,
        source_service_name:,
        destination_service_name:,
        phase: DIRECTIONS.fetch(direction),
        **counts
      )
      emit(result)
      result
    end

    private

    attr_reader :source_service_name, :destination_service_name, :blob_scope, :service_registry,
                :batch_size, :run_id, :copy_missing

    def validate!
      raise ArgumentError, 'unsupported storage migration direction' unless DIRECTIONS.key?(direction)
      raise ArgumentError, 'batch_size must be positive' unless batch_size.positive?

      source_service
      destination_service
    end

    def direction
      [source_service_name, destination_service_name]
    end

    def source_service
      service_registry.fetch(source_service_name)
    end

    def destination_service
      service_registry.fetch(destination_service_name)
    end

    def process(blob, counts)
      counts[:processed_count] += 1
      if destination_service.exist?(blob.key)
        verify(blob, destination_service)
      elsif copy_missing
        copy(blob)
        verify(blob, destination_service)
      else
        raise ActiveStorage::FileNotFoundError
      end
      counts[:verified_count] += 1
    rescue StandardError
      counts[:failed_count] += 1
    end

    def copy(blob)
      source_service.open(blob.key, checksum: blob.checksum, verify: true) do |source_file|
        destination_service.upload(blob.key, source_file, checksum: blob.checksum)
      end
    end

    def verify(blob, service)
      service.open(blob.key, checksum: blob.checksum, verify: true) { |file| file.read(1) }
    end

    def emit(result)
      Observability::StorageEvent.migration(
        service: DIRECTIONS.fetch(direction),
        outcome: result.successful? ? :success : :failure,
        reason: result.successful? ? :completed : :failed,
        counts: result.to_h.slice(:processed_count, :verified_count, :failed_count)
      )
    end
  end
end
