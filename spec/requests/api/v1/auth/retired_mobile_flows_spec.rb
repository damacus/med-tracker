# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Retired native authentication flows' do
  before { pending 'Legacy removal blocked by automatic approval review; see retirement-approval.md' }

  %w[login oidc_exchange select_household refresh].each do |action|
    it "does not route the old #{action} endpoint" do
      expect(post: "/api/v1/auth/#{action}").not_to be_routable
    end
  end

  it 'advertises Rodauth account login without legacy exchange or password issuance' do
    get '/api/v1/capabilities', as: :json
    authentication = response.parsed_body.dig('data', 'authentication')
    expect(authentication).not_to include('oidc_exchange', 'password_login')
    expect(authentication.fetch('hosted_mobile')).to eq('rodauth_authorization_code_pkce')
  end
end
