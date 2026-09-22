require 'rails_helper'

RSpec.describe Canary::MobileClientRegistration do
  let(:connection) { ActiveRecord::Base.connection }
  let(:targets) do
    {
      demo_mode: true,
      application_url: 'https://med-tracker-canary.damacus.io',
      database_hosts: ['med-tracker-canary-rw.home.svc.cluster.local'],
      database_role: 'med_tracker_owner'
    }
  end

  it 'registers exact clients without changing unrelated integrations or grants' do
    existing = OauthApplication.create!(name: 'Other', client_id: 'other', redirect_uri: 'https://example.test/callback',
                                        scopes: 'patient/*.rs', token_endpoint_auth_method: 'none')
    allow(connection).to receive(:execute).and_call_original

    described_class.new(targets:, connection:).call
    described_class.new(targets:, connection:).call

    expect(OauthApplication.where(client_id: 'medtracker-ios-staging').count).to eq(1)
    expect(existing.reload.name).to eq('Other')
    expect(connection).to have_received(:execute).with(DemoReset::PrimaryDatabaseReset::ADVISORY_LOCK_SQL).twice
  end

  it 'refuses a conflicting known client and rolls back earlier inserts' do
    OauthApplication.create!(name: 'Wrong', client_id: 'medtracker-ios-staging',
                             redirect_uri: 'io.damacus.medtracker.staging:/oauth2redirect',
                             scopes: 'medtracker offline_access', client_kind: 'mobile',
                             token_endpoint_auth_method: 'none')

    expect { described_class.new(targets:, connection:).call }
      .to raise_error(DemoBaseline::PublicMobileClients::MismatchedClientError)
    expect(OauthApplication.where(client_id: 'io.damacus.medtracker')).not_to exist
  end

  {
    demo_mode: false,
    application_url: 'https://med-tracker.damacus.io',
    database_hosts: ['med-tracker-canary-rw.home.svc.cluster.local', 'other-db'],
    database_role: 'med_tracker_app'
  }.each do |field, value|
    it "refuses unsafe #{field} without mutation" do
      expect { described_class.new(targets: targets.merge(field => value), connection:).call }
        .to raise_error(DemoReset::UnsafeTargetError, /#{field}/)
      expect(OauthApplication.where(client_id: 'medtracker-ios-staging')).not_to exist
    end
  end
end
