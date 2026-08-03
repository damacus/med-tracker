# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::Preflight do
  subject(:preflight) { described_class.new(**targets) }

  let(:targets) do
    {
      demo_mode: true,
      application_url: 'https://med-tracker-canary.damacus.io',
      database_host: 'med-tracker-canary-rw.home.svc.cluster.local',
      storage_root: '/app/storage',
      database_role: 'med_tracker_owner'
    }
  end

  it 'accepts only the complete canary target boundary' do
    expect(preflight.call).to eq(
      outcome: 'passed',
      targets: %w[demo_mode application_host database_host storage_root database_role]
    )
  end

  it 'refuses when any configured auxiliary database names another target' do
    mixed_hosts = [
      'med-tracker-canary-rw.home.svc.cluster.local',
      'med-tracker-rw.home.svc.cluster.local'
    ]

    expect { described_class.new(**targets, database_host: mixed_hosts).call }
      .to raise_error(DemoReset::UnsafeTargetError, /database_host/)
  end

  {
    demo_mode: false,
    application_url: 'https://med-tracker.damacus.io',
    database_host: 'med-tracker-rw.home.svc.cluster.local',
    storage_root: '/app/production-storage',
    database_role: 'med_tracker_app'
  }.each do |target, unsafe_value|
    it "refuses a mismatched #{target} without mutating data or exposing the value" do
      account = Account.create!(email: "preflight-#{target}@example.test", status: :verified)
      unsafe_targets = targets.merge(target => unsafe_value)
      original_updated_at = account.updated_at

      expect { described_class.new(**unsafe_targets).call }
        .to raise_error(DemoReset::UnsafeTargetError, /#{target.to_s.sub('application_url', 'application_host')}/)
      expect(account.reload.updated_at).to eq(original_updated_at)

      expect do
        described_class.new(**unsafe_targets).call
      end.to(raise_error { |error| expect(error.message).not_to include(unsafe_value.to_s) })
    end
  end
end
