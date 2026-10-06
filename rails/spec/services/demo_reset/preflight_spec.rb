# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::Preflight do
  subject(:preflight) { described_class.new(expected: expected_targets, **targets) }

  let(:targets) do
    {
      demo_mode: true,
      application_url: 'https://demo.example.test',
      database_host: 'demo-database.example.test',
      storage_service: 's3',
      storage_root: nil,
      storage_endpoint: 'https://objects.example.test',
      storage_bucket: 'demo-archive',
      database_role: 'demo_owner'
    }
  end
  let(:expected_targets) do
    {
      application_host: 'demo.example.test',
      database_host: 'demo-database.example.test',
      storage_service: 's3',
      storage_root: nil,
      storage_endpoint: 'https://objects.example.test',
      storage_bucket: 'demo-archive',
      database_role: 'demo_owner'
    }
  end

  it 'accepts only the complete canary target boundary' do
    expect(preflight.call).to eq(
      outcome: 'passed',
      targets: %w[
        demo_mode application_host database_host storage_service storage_endpoint storage_bucket database_role
      ]
    )
  end

  it 'refuses when any configured auxiliary database names another target' do
    mixed_hosts = [
      'demo-database.example.test',
      'another-database.example.test'
    ]

    expect { described_class.new(expected: expected_targets, **targets, database_host: mixed_hosts).call }
      .to raise_error(DemoReset::UnsafeTargetError, /database_host/)
  end

  it 'accepts the expected persistent storage root without requiring S3 settings' do
    disk_targets = targets.merge(
      storage_service: 'persistent', storage_root: '/app/storage', storage_endpoint: nil, storage_bucket: nil
    )
    expected_disk_targets = expected_targets.merge(
      storage_service: 'persistent', storage_root: '/app/storage', storage_endpoint: nil, storage_bucket: nil
    )

    expect(described_class.new(expected: expected_disk_targets, **disk_targets).call).to eq(
      outcome: 'passed',
      targets: %w[demo_mode application_host database_host storage_service storage_root database_role]
    )
  end

  it 'requires both backend boundaries for a mirrored storage service' do
    mirror_targets = targets.merge(storage_service: 'persistent_with_s3_mirror', storage_root: '/app/storage')
    expected_mirror_targets = expected_targets.merge(
      storage_service: 'persistent_with_s3_mirror', storage_root: '/app/storage'
    )

    expect(described_class.new(expected: expected_mirror_targets, **mirror_targets).call).to eq(
      outcome: 'passed',
      targets: %w[
        demo_mode application_host database_host storage_service storage_root storage_endpoint storage_bucket
        database_role
      ]
    )
  end

  it 'refuses a persistent root outside the expected storage boundary' do
    disk_targets = targets.merge(
      storage_service: 'persistent', storage_root: '/other/storage', storage_endpoint: nil, storage_bucket: nil
    )
    expected_disk_targets = expected_targets.merge(
      storage_service: 'persistent', storage_root: '/app/storage', storage_endpoint: nil, storage_bucket: nil
    )

    expect { described_class.new(expected: expected_disk_targets, **disk_targets).call }
      .to raise_error(DemoReset::UnsafeTargetError, /storage_root/)
  end

  {
    demo_mode: false,
    application_url: 'https://another.example.test',
    database_host: 'another-database.example.test',
    storage_service: 'persistent',
    storage_endpoint: 'https://other-objects.example.test',
    storage_bucket: 'other-archive',
    database_role: 'runtime_user'
  }.each do |target, unsafe_value|
    it "refuses a mismatched #{target} without mutating data or exposing the value" do
      account = Account.create!(email: "preflight-#{target}@example.test", status: :verified)
      unsafe_targets = targets.merge(target => unsafe_value)
      original_updated_at = account.updated_at

      expect { described_class.new(expected: expected_targets, **unsafe_targets).call }
        .to raise_error(DemoReset::UnsafeTargetError, /#{target.to_s.sub('application_url', 'application_host')}/)
      expect(account.reload.updated_at).to eq(original_updated_at)

      expect do
        described_class.new(expected: expected_targets, **unsafe_targets).call
      end.to(raise_error { |error| expect(error.message).not_to include(unsafe_value.to_s) })
    end
  end
end
