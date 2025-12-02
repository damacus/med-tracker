# frozen_string_literal: true

require 'rails_helper'

# Request specs for Admin Audit Logs
# These test HTTP-level behavior (status codes, redirects, response format)
# Detailed content/UI testing is in spec/features/admin/audit_logs_spec.rb
RSpec.describe 'Admin::AuditLogs' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }
  let(:carer) { users(:bob) }
  let(:prescription) { prescriptions(:john_paracetamol) }

  # Helper to sign in via Rodauth
  def sign_in_as(user)
    account = Account.find_by(email: user.email_address)
    post '/login', params: { email: account.email, password: 'password' }
    follow_redirect! if response.redirect?
  end

  describe 'GET /admin/audit_logs' do
    context 'when authenticated as administrator' do
      before { sign_in_as(admin) }

      it 'returns success' do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      it 'returns HTML content type' do
        get admin_audit_logs_path
        expect(response.content_type).to include('text/html')
      end
    end

    context 'when authenticated as non-administrator' do
      before { sign_in_as(regular_user) }

      it 'denies access' do
        get admin_audit_logs_path
        expect(response).to redirect_to(root_path)
      end
    end

    context 'when not authenticated' do
      it 'redirects to login' do
        get admin_audit_logs_path
        expect(response).to redirect_to(login_path)
      end
    end
  end

  describe 'GET /admin/audit_logs with filters' do
    before do
      sign_in_as(admin)
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end
    end

    it 'accepts item_type filter parameter' do
      get admin_audit_logs_path, params: { item_type: 'User' }
      expect(response).to have_http_status(:success)
    end

    it 'accepts event filter parameter' do
      get admin_audit_logs_path, params: { event: 'update' }
      expect(response).to have_http_status(:success)
    end

    it 'accepts page parameter' do
      get admin_audit_logs_path, params: { page: 1 }
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /admin/audit_logs/:id' do
    let!(:version) do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end
      PaperTrail::Version.last
    end

    context 'when authenticated as administrator' do
      before { sign_in_as(admin) }

      it 'returns success' do
        get admin_audit_log_path(version)
        expect(response).to have_http_status(:success)
      end

      it 'returns HTML content type' do
        get admin_audit_log_path(version)
        expect(response.content_type).to include('text/html')
      end
    end

    context 'when authenticated as non-administrator' do
      before { sign_in_as(regular_user) }

      it 'denies access' do
        get admin_audit_log_path(version)
        # May redirect to root_path or otp-setup depending on 2FA requirements
        expect(response).to have_http_status(:redirect)
        expect(response).not_to have_http_status(:success)
      end
    end

    context 'when not authenticated' do
      it 'redirects to login' do
        get admin_audit_log_path(version)
        expect(response).to redirect_to(login_path)
      end
    end
  end
end
