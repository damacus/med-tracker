# frozen_string_literal: true

module NhsDmd
  class ArchiveStore
    class Error < StandardError; end

    Reference = Data.define(:service_name, :key, :checksum, :byte_size)

    def initialize(service_registry: ActiveStorage::Blob.services)
      @service_registry = service_registry
    end

    def persist(import_run:, uploaded_file:, service_name: Rails.configuration.active_storage.service)
      path = upload_path(uploaded_file)
      reference = build_reference(path, service_name)
      service = fetch_service(service_name)
      upload_and_verify(service, path, reference)
      attach_reference(import_run, reference)
      reference
    rescue StandardError
      service&.delete(reference.key) if service && reference
      raise Error, 'archive_persistence_failed'
    end

    def open(import_run)
      if import_run.archive_reference?
        service = fetch_service(import_run.archive_service_name)
        service.open(import_run.archive_key, checksum: import_run.archive_checksum, verify: true) do |file|
          return yield(file.path)
        end
      end
      return yield(import_run.archive_path) if import_run.legacy_archive?

      raise Error, 'archive_reference_missing'
    rescue Error
      raise
    rescue StandardError
      raise Error, 'archive_read_failed'
    end

    def cleanup(import_run)
      raise Error, 'archive_not_terminal' if import_run.active?

      fetch_service(import_run.archive_service_name).delete(import_run.archive_key) if import_run.archive_reference?
      FileUtils.rm_f(import_run.archive_path) if import_run.legacy_archive?
      import_run.update!(
        archive_service_name: nil,
        archive_key: nil,
        archive_checksum: nil,
        archive_byte_size: nil,
        archive_path: nil
      )
    rescue Error
      raise
    rescue StandardError
      raise Error, 'archive_cleanup_failed'
    end

    def convert_legacy(import_run:, service_name: Rails.configuration.active_storage.service)
      raise Error, 'legacy_archive_missing' unless import_run.active? && import_run.legacy_archive?

      legacy_path = import_run.archive_path
      reference = persist(import_run:, uploaded_file: legacy_path, service_name:)
      FileUtils.rm_f(legacy_path)
      reference
    end

    private

    attr_reader :service_registry

    def fetch_service(service_name)
      service_registry.fetch(service_name.to_sym)
    rescue KeyError
      raise Error, 'archive_service_unavailable'
    end

    def upload_path(uploaded_file)
      path = uploaded_file.respond_to?(:path) ? uploaded_file.path : uploaded_file.to_s
      raise Error, 'archive_missing' if path.blank? || !File.file?(path)

      path
    end

    def build_reference(path, service_name)
      Reference.new(
        service_name: service_name.to_s,
        key: "nhs-dmd/imports/#{SecureRandom.uuid}",
        checksum: Digest::MD5.file(path).base64digest,
        byte_size: File.size(path)
      )
    end

    def upload_and_verify(service, path, reference)
      File.open(path, 'rb') { service.upload(reference.key, it, checksum: reference.checksum) }
      service.open(reference.key, checksum: reference.checksum, verify: true) { it.read(1) }
    end

    def attach_reference(import_run, reference)
      import_run.update!(
        archive_service_name: reference.service_name,
        archive_key: reference.key,
        archive_checksum: reference.checksum,
        archive_byte_size: reference.byte_size,
        archive_path: nil
      )
    end
  end
end
