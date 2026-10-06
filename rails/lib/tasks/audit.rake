# frozen_string_literal: true

namespace :audit do
  desc 'Verify audit evidence with SCOPE=database|worm|combined and FORMAT=human|json'
  task verify: :environment do
    scope = ENV.fetch('SCOPE', 'database').downcase
    worm_verifier = Audit::Verification::WormVerifierFactory.new.call if %w[worm combined].include?(scope)
    status = Audit::Verification::Command.new(worm_verifier:).call
    exit(status) unless status.zero?
  end

  desc 'Export native NDJSON, a signed manifest, and optional FHIR R4 AuditEvent resources'
  task export: :environment do
    filter = Audit::EntryFilter.new(ENV.to_h)
    signer = Audit::ManifestSigner.new(
      key_id: ENV.fetch('AUDIT_MANIFEST_SIGNING_KEY_ID'),
      private_key_pem: audit_manifest_private_key
    )
    result = Audit::EvidenceExporter.new(
      entries: filter.call, output_directory: ENV.fetch('OUTPUT', 'tmp/audit-exports/latest'),
      manifest_signer: signer, household_id: ENV['HOUSEHOLD_ID'].presence&.to_i,
      fhir: ActiveModel::Type::Boolean.new.cast(ENV.fetch('FHIR', nil))
    ).call
    puts({ native: result.native_path.to_s, manifest: result.manifest_path.to_s,
           fhir: result.fhir_path&.to_s }.compact.to_json)
  rescue KeyError, Audit::EntryFilter::Invalid, ArgumentError => e
    warn("audit export could not run: #{e.message}")
    exit(2)
  end

  desc 'Emit aggregate audit-delivery backlog status'
  task monitor: :environment do
    result = Audit::BacklogMonitor.new.call
    puts({ severity: result.severity, pending_count: result.pending_count,
           oldest_age_seconds: result.oldest_age_seconds }.to_json)
    exit(1) if result.severity == 'critical'
  end

  def audit_manifest_private_key
    path = ENV.fetch('AUDIT_MANIFEST_SIGNING_PRIVATE_KEY_FILE', nil)
    return File.binread(path) if path.present?

    ENV.fetch('AUDIT_MANIFEST_SIGNING_PRIVATE_KEY')
  end
end
