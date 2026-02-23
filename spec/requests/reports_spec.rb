# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Reports' do
  fixtures :all

  let(:user) { users(:admin) }

  describe 'GET /reports' do
    context 'when user is not authenticated' do
      it 'redirects to the login page' do
        get reports_path
        expect(response).to redirect_to(login_path)
      end
    end

    context 'when user is authenticated and authorized' do
      before do
        post '/login', params: { email: user.email_address, password: 'password' }
      end

      it 'returns HTTP success' do
        get reports_path
        expect(response).to have_http_status(:success)
      end

      it 'returns HTTP success when applying date filters' do
        get reports_path, params: { start_date: '2023-01-01', end_date: '2023-01-31' }
        expect(response).to have_http_status(:success)
      end

      it 'redirects with alert when providing invalid date formats' do
        get reports_path, params: { start_date: 'invalid-date' }
        expect(response).to redirect_to(reports_path)
        expect(flash[:alert]).to eq('Invalid date format provided.')
      end
    end

    context 'when user is authenticated but not authorized' do
      let(:user) { users(:minor_patient_user) } # minor

      before do
        post '/login', params: { email: user.email_address, password: 'password' }
      end

      it 'redirects to root path' do
        get reports_path
        expect(response).to redirect_to(root_path)
        expect(flash[:alert]).to eq('You are not authorized to perform this action.')
      end
    end
  end
end
