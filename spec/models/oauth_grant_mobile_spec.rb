# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OauthGrant do
  fixtures :accounts

  let(:application) do
    OauthApplication.create!(name: 'Android', client_id: 'mobile-android', client_kind: 'mobile',
                             redirect_uri: 'io.damacus.medtracker:/oauth2redirect',
                             scopes: 'medtracker offline_access', token_endpoint_auth_method: 'none')
  end
  let(:grant) do
    described_class.new(account: accounts(:jane_doe), oauth_application: application, client_kind: 'mobile',
                        scopes: 'medtracker offline_access', expires_in: 15.minutes.from_now,
                        authenticated_at: Time.current, last_used_at: Time.current)
  end

  it 'persists an explicit account grant without household or person binding' do
    expect(grant.save).to be(true)
    expect(grant.reload).to be_mobile
    expect(grant.household_membership).to be_nil
  end

  it 'rejects an unclassified grant without the restricted credential fields' do
    grant.client_kind = 'integration'

    expect(grant).not_to be_valid
    expect { grant.save!(validate: false) }.to raise_error(ActiveRecord::StatementInvalid)
  end

  it 'rejects a mobile grant issued by a restricted integration application' do
    application.update!(client_kind: 'integration', redirect_uri: 'https://client.example/callback',
                        scopes: 'patient/*.rs offline_access')

    expect(grant).not_to be_valid
    expect { grant.save!(validate: false) }.to raise_error(ActiveRecord::StatementInvalid)
  end

  it 'does not allow first-party registration to request SMART patient scopes' do
    application.scopes = 'medtracker patient/*.rs'

    expect(application).not_to be_valid
  end

  it 'does not allow restricted registrations to request first-party API scope' do
    application.client_kind = 'integration'
    application.redirect_uri = 'https://client.example/callback'

    expect(application).not_to be_valid
  end

  it 'requires native clients to remain public clients' do
    application.client_secret_hash = 'hashed-secret'
    application.token_endpoint_auth_method = 'client_secret_post'

    expect(application).not_to be_valid
  end
end
