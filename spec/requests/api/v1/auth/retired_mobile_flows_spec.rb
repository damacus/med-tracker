# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Retired native authentication flows' do
  %w[login oidc_exchange select_household refresh].each do |action|
    it "does not route the old #{action} endpoint" do
      expect do
        Rails.application.routes.recognize_path("/api/v1/auth/#{action}", method: :post)
      end.to raise_error(ActionController::RoutingError)
    end
  end

  it 'advertises Rodauth account login without legacy exchange or password issuance' do
    get '/api/v1/capabilities', as: :json
    authentication = response.parsed_body.dig('data', 'authentication')
    expect(authentication).not_to include('oidc_exchange', 'password_login')
    expect(authentication.fetch('hosted_mobile')).to eq('rodauth_authorization_code_pkce')
  end
end
