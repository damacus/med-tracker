# frozen_string_literal: true

require 'rails_helper'
require 'tmpdir'

RSpec.describe Audit::EvidenceExporter do
  fixtures :accounts, :people, :users

  let(:household) { users(:admin).person.household }
  let(:manifest_signer) do
    private_key = OpenSSL::PKey.generate_key('ED25519')
    Audit::ManifestSigner.new(key_id: 'export-key-2026-01', private_key_pem: private_key.private_to_pem)
  end
  let(:directory) { Pathname(Dir.mktmpdir('audit-export')) }
  let(:result) do
    described_class.new(
      entries: AuditLedgerEntry.where(household:), output_directory: directory,
      manifest_signer:, household_id: household.id, fhir: true,
      clock: -> { Time.iso8601('2026-07-09T12:30:00Z') }
    ).call
  end
  let(:manifest) { JSON.parse(File.binread(result.manifest_path)) }

  before { Audit::Event.record!(household:, event_type: 'audit.export.subject', metadata: { outcome: 'success' }) }

  after { FileUtils.remove_entry(directory) if directory.exist? }

  it 'writes deterministic native evidence and audits the export' do
    native_lines = File.readlines(result.native_path, chomp: true).map { |line| JSON.parse(line) }

    expect(native_lines).not_to be_empty
    expect(native_lines).to all(include('evidence_type' => 'ledger_entry'))
    expect(SecurityAuditEvent.where(event_type: 'audit.evidence.exported', household:)).to exist
  end

  it 'does not record a successful export when evidence cannot be written' do
    allow(File).to receive(:binwrite).and_raise(Errno::ENOSPC)

    expect { result }.to raise_error(Errno::ENOSPC)
    expect(SecurityAuditEvent.where(event_type: 'audit.evidence.exported', household:)).not_to exist
  end

  it 'writes a signed manifest for the native evidence' do
    expect(manifest.dig('files', 'audit.ndjson', 'sha256')).to eq(Digest::SHA256.file(result.native_path).hexdigest)
    expect(manifest.dig('signing', 'key_id')).to eq('export-key-2026-01')
    expect(manifest_signature_valid?(manifest)).to be(true)
  end

  it 'optionally writes a FHIR R4 AuditEvent bundle' do
    bundle = JSON.parse(File.binread(result.fhir_path))

    expect(bundle).to include('resourceType' => 'Bundle', 'type' => 'collection')
    expect(bundle.fetch('entry')).to all(satisfy { |item| item.dig('resource', 'resourceType') == 'AuditEvent' })
  end

  private

  def manifest_signature_valid?(manifest)
    signing = manifest.fetch('signing')
    unsigned = manifest.except('signing')
    public_key = OpenSSL::PKey.read(Base64.strict_decode64(signing.fetch('public_key')))
    public_key.verify(
      nil, Base64.strict_decode64(signing.fetch('signature')),
      Audit::ManifestSigner.canonical_json(unsigned)
    )
  end
end
