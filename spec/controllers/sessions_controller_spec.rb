# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SessionsController, type: :controller do
  describe 'GET #new' do
    it 'returns a successful response' do
      get :new
      expect(response).to have_http_status(:ok)
    end
  end
  
  describe 'POST #create' do
    let(:user) { User.create!(name: 'Test User', email_address: 'unique-test@example.com', password: 'password', date_of_birth: 30.years.ago) }
    
    context 'with valid credentials' do
      it 'creates a new user session and redirects' do
        # The app uses a Session model and cookies, so we need to test differently
        expect do
          post :create, params: { email_address: user.email_address, password: 'password' }
        end.to change { user.sessions.count }.by(1)
        
        expect(response).to redirect_to(root_path)
      end
    end
    
    context 'with invalid credentials' do
      it 'redirects to login with error' do
        post :create, params: { email_address: user.email_address, password: 'wrong' }
        
        # No new session should be created
        expect(user.sessions.count).to eq(0)
        
        # Should redirect back to login
        expect(response).to have_http_status(:redirect)
        expect(response).to redirect_to(new_session_path(email_address: user.email_address))
      end
    end
  end
end
