# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::FhirAuditEventMapper do
  let(:entry) do
    instance_double(
      AuditLedgerEntry,
      envelope: {
        'event_id' => 'aaf4883c-a035-4ff0-a96d-df2f4a3597a0',
        'event_type' => 'medication.read', 'outcome' => 'success',
        'occurred_at' => '2026-07-09T12:00:00.000000Z',
        'agent' => { 'account_id' => 42, 'role' => 'administrator' },
        'policy' => { 'class' => 'MedicationPolicy', 'query' => 'show?' },
        'request' => { 'ip' => '192.0.2.10' },
        'source' => { 'table' => 'versions', 'id' => 9 },
        'entity' => { 'type' => 'Medication', 'id' => 7 }
      }
    )
  end

  it 'maps the native envelope to a valid-shaped FHIR R4 AuditEvent' do
    resource = described_class.new(entry).to_h

    expect(resource).to include(
      resourceType: 'AuditEvent', id: 'aaf4883c-a035-4ff0-a96d-df2f4a3597a0',
      action: 'R', recorded: '2026-07-09T12:00:00.000000Z', outcome: '0',
      type: hash_including(code: '110100'),
      source: hash_including(observer: hash_including(identifier: hash_including(value: 'medtracker')))
    )
    expect(resource.dig(:agent, 0, :requestor)).to be(true)
    expect(resource.dig(:agent, 0, :role, 0, :coding, 0, :code)).to eq('administrator')
    expect(resource.dig(:entity, 0, :what, :identifier, :value)).to eq('versions/9')
  end
end
