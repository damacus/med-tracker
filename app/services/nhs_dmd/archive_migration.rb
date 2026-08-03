# frozen_string_literal: true

module NhsDmd
  class ArchiveMigration
    Result = Data.define(:processed_count, :converted_count, :failed_count, :applied)
    DEFAULT_BATCH_SIZE = 100

    def initialize(scope: NhsDmdImport.all, store: ArchiveStore.new, **options)
      @scope = scope
      @store = store
      @owner_role = options.fetch(:owner_role) do
        -> { ActiveRecord::Base.connection.select_value('SELECT current_user') == 'med_tracker_owner' }
      end
      @path_exists = options.fetch(:path_exists, ->(path) { File.file?(path) })
      @service_name = options.fetch(:service_name, Rails.configuration.active_storage.service).to_sym
      @batch_size = options.fetch(:batch_size, DEFAULT_BATCH_SIZE)
      @apply = options.fetch(:apply, false)
    end

    def call
      raise SecurityError, 'archive migration requires the owner database role' unless owner_role.call
      raise ArgumentError, 'unsupported archive service' unless ProductionStorage::SERVICES.include?(service_name.to_s)

      counts = { processed_count: 0, converted_count: 0, failed_count: 0 }
      legacy_scope.find_in_batches(batch_size:) do |imports|
        imports.each { process(it, counts) }
      end
      Result.new(**counts, applied: apply)
    end

    private

    attr_reader :scope, :store, :owner_role, :path_exists, :service_name, :batch_size, :apply

    def legacy_scope
      scope.where.not(status: NhsDmdImport.statuses.values_at('completed', 'failed'))
           .where(archive_key: nil)
           .where.not(archive_path: [nil, ''])
    end

    def process(import_run, counts)
      return unless import_run.active?

      counts[:processed_count] += 1
      unless path_exists.call(import_run.archive_path)
        counts[:failed_count] += 1
        return
      end
      return unless apply

      store.convert_legacy(import_run:, service_name:)
      counts[:converted_count] += 1
    rescue StandardError
      counts[:failed_count] += 1
    end
  end
end
