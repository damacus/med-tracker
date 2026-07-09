# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::ObjectLock::Configuration do
  subject(:configuration) { described_class.new(environment) }

  let(:environment) do
    {
      'AUDIT_WORM_BUCKET' => 'medtracker-audit',
      'AUDIT_WORM_REGION' => 'eu-west-2',
      'AUDIT_WORM_EXPECTED_OWNER' => '123456789012',
      'AUDIT_WORM_RETENTION_MODE' => 'GOVERNANCE',
      'AUDIT_WORM_SSE' => 'aws:kms',
      'AUDIT_WORM_KMS_KEY_ID' => 'alias/medtracker-audit'
    }
  end

  it 'loads a governance configuration without exposing credentials' do
    expect(configuration).to have_attributes(
      bucket: 'medtracker-audit', region: 'eu-west-2', expected_owner: '123456789012',
      retention_mode: 'GOVERNANCE', server_side_encryption: 'aws:kms'
    )
    expect(configuration.to_h.keys).not_to include(:access_key_id, :secret_access_key)
  end

  it 'rejects compliance retention without recorded governance approval' do
    environment['AUDIT_WORM_RETENTION_MODE'] = 'COMPLIANCE'

    expect { configuration }.to raise_error(described_class::Invalid, /COMPLIANCE.*approval/)
  end

  it 'accepts compliance retention only with explicit approval' do
    environment['AUDIT_WORM_RETENTION_MODE'] = 'COMPLIANCE'
    environment['AUDIT_WORM_COMPLIANCE_APPROVED'] = 'true'

    expect(configuration.retention_mode).to eq('COMPLIANCE')
  end

  it 'requires a KMS key for KMS encryption' do
    environment.delete('AUDIT_WORM_KMS_KEY_ID')

    expect { configuration }.to raise_error(described_class::Invalid, /KMS key/)
  end
end
