# frozen_string_literal: true

module Storage
  class RecoverySet
    class Error < StandardError; end

    Result = Data.define(:database_reference, :storage_references, :migration_phase)
    BACKENDS = {
      'test' => %i[disk],
      'local' => %i[disk],
      'persistent' => %i[disk],
      'persistent_with_s3_mirror' => %i[disk s3],
      's3_with_persistent_mirror' => %i[disk s3],
      's3' => %i[s3]
    }.freeze

    def initialize(blob_scope: ActiveStorage::Blob.all, migration_run: nil, backends: nil)
      @blob_scope = blob_scope
      @migration_run = migration_run
      @backends = backends
    end

    def required_backends
      required = backends || service_names.flat_map { BACKENDS.fetch(it) }.uniq
      required |= %i[disk s3] if migration_run && !migration_run.finalized?
      required.sort_by { %i[disk s3].index(it) }
    rescue KeyError
      raise Error, 'unsupported_blob_service'
    end

    def record(database_reference:, disk_reference: nil, s3_reference: nil)
      raise Error, 'database_recovery_reference_required' if database_reference.blank?

      references = {}
      required_backends.each do |backend|
        reference = backend == :disk ? disk_reference : s3_reference
        raise Error, "#{backend}_recovery_reference_required" if reference.blank?

        references[backend] = reference
      end
      Result.new(
        database_reference:,
        storage_references: references,
        migration_phase: migration_run&.phase
      )
    end

    private

    attr_reader :blob_scope, :migration_run, :backends

    def service_names
      blob_scope.distinct.pluck(:service_name)
    end
  end
end
