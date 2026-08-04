# frozen_string_literal: true

module DemoReset
  class StorageCleaner
    BACKENDS = {
      'persistent' => %i[disk],
      's3' => %i[s3],
      'persistent_with_s3_mirror' => %i[disk s3],
      's3_with_persistent_mirror' => %i[disk s3]
    }.freeze

    def initialize(service_name: Rails.application.config.active_storage.service,
                   expected_service_name: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_SERVICE', nil),
                   disk_cleaner: nil, s3_cleaner: nil)
      @service_name = service_name.to_s
      @expected_service_name = expected_service_name.to_s
      @disk_cleaner = disk_cleaner
      @s3_cleaner = s3_cleaner
    end

    def call
      active_cleaners.each_with_object({}) do |cleaner, result|
        result.merge!(cleaner.call)
      end
    end

    def empty?
      active_cleaners.all?(&:empty?)
    end

    private

    attr_reader :service_name, :expected_service_name

    def active_cleaners
      verify_service!
      BACKENDS.fetch(service_name).map { |backend| cleaner_for(backend) }
    end

    def verify_service!
      valid = ProductionStorage::SERVICES.include?(service_name) && service_name == expected_service_name
      raise UnsafeTargetError, 'demo reset refused: storage_service' unless valid
    end

    def cleaner_for(backend)
      return disk_cleaner if backend == :disk

      s3_cleaner
    end

    def disk_cleaner
      @disk_cleaner ||= DiskStorageCleaner.new
    end

    def s3_cleaner
      @s3_cleaner ||= S3StorageCleaner.new
    end
  end
end
