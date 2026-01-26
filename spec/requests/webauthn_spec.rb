# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'WebAuthn API' do
  fixtures :all

  let(:account) { accounts(:damacus) }

  describe 'GET /webauthn/registration-options' do
    context 'when authenticated' do
      before do
        post '/login', params: { email: account.email, password: 'password' }
      end

      it 'returns valid registration options' do
        get '/webauthn/registration-options', headers: { 'Accept' => 'application/json' }

        expect(response).to have_http_status(:ok)
        json = response.parsed_body
        expect(json['challenge']).to be_present
        expect(json['rp']['name']).to eq('MedTracker')
        expect(json['user']['id']).to eq(account.id.to_s)
      end
    end

    context 'when not authenticated' do
      it 'redirects to login' do
        get '/webauthn/registration-options', headers: { 'Accept' => 'application/json' }

        expect(response).to have_http_status(:redirect)
      end
    end
  end

  describe 'GET /webauthn/authentication-options' do
    it 'returns valid authentication options without authentication' do
      get '/webauthn/authentication-options', headers: { 'Accept' => 'application/json' }

      expect(response).to have_http_status(:ok)
      json = response.parsed_body
      expect(json['challenge']).to be_present
      expect(json['userVerification']).to eq('required')
    end

    it 'stores challenge in session' do
      get '/webauthn/authentication-options', headers: { 'Accept' => 'application/json' }

      expect(session[:webauthn_challenge]).to be_present
    end
  end
end
