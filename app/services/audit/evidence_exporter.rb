# frozen_string_literal: true

require 'digest'
require 'fileutils'
require 'json'

module Audit
  class EvidenceExporter
    Result = Data.define(:native_path, :manifest_path, :fhir_path)

    def initialize(entries:, output_directory:, manifest_signer:, **options)
      @entries_source = entries
      @output_directory = Pathname(output_directory)
      @manifest_signer = manifest_signer
      @household_id = options[:household_id]
      @fhir = options.fetch(:fhir, false)
      @clock = options.fetch(:clock, -> { Time.current })
    end

    def call
      FileUtils.mkdir_p(output_directory)
      write_native
      write_fhir if fhir
      write_manifest
      record_export_event
      Result.new(native_path:, manifest_path:, fhir_path: fhir ? fhir_path : nil)
    end

    private

    attr_reader :entries_source, :output_directory, :manifest_signer, :household_id, :fhir, :clock

    def entries
      @entries ||= entries_source.reorder(:chain_key, :chain_epoch, :sequence).to_a
    end

    def record_export_event
      Audit::Event.record!(
        household_id:, event_type: 'audit.evidence.exported',
        metadata: { outcome: 'success', format: fhir ? 'native-and-fhir-r4' : 'native' }
      )
    end

    def write_native
      body = entries.map { |entry| Audit::ObjectLock::RecordSerializer.new(entry).body }.join("\n")
      body += "\n" if body.present?
      File.binwrite(native_path, body)
    end

    def write_fhir
      bundle = {
        resourceType: 'Bundle', type: 'collection', timestamp: generated_at,
        entry: entries.map do |entry|
          resource = FhirAuditEventMapper.new(entry).to_h
          { fullUrl: "urn:uuid:#{resource[:id]}", resource: }
        end
      }
      File.binwrite(fhir_path, ManifestSigner.canonical_json(bundle))
    end

    def write_manifest
      unsigned = {
        schema_version: 1, generated_at:, household_id:,
        files: exported_files.to_h { |path| [path.basename.to_s, file_description(path)] },
        chains: chain_descriptions
      }.compact
      File.binwrite(manifest_path, ManifestSigner.canonical_json(manifest_signer.sign(unsigned)))
    end

    def exported_files
      [native_path, (fhir_path if fhir)].compact
    end

    def file_description(path)
      { sha256: Digest::SHA256.file(path).hexdigest, bytes: path.size }
    end

    def chain_descriptions
      entries.group_by { |entry| [entry.chain_key, entry.chain_epoch] }.map do |(chain_key, epoch), values|
        tail = values.max_by(&:sequence)
        { chain_key:, epoch:, final_sequence: tail.sequence, final_hash: tail.entry_hash.unpack1('H*') }
      end
    end

    def generated_at
      @generated_at ||= clock.call.utc.iso8601(6)
    end

    def native_path
      output_directory.join('audit.ndjson')
    end

    def fhir_path
      output_directory.join('fhir-audit-events.json')
    end

    def manifest_path
      output_directory.join('manifest.json')
    end
  end
end
