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
  let(:schedule) { schedules(:john_paracetamol) }

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
  end

  describe 'GET /admin/audit_logs' do
    context 'when authenticated as administrator' do
      before { sign_in(admin) }

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
      before { sign_in(regular_user) }

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
      sign_in(admin)
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
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

    it 'surfaces filter options for all PaperTrail item types and events' do
      insert_unlisted_audit_record

      get admin_audit_logs_path

      expect(response.body).to include('Unlisted Audit Record')
      expect(response.body).to include('Custom/Audit Event')
    end

    it 'does not include another household audit records' do
      foreign_version = create_audit_version(household: other_household, event: 'foreign/audit_event')

      get admin_audit_logs_path

      expect(response.body).not_to include(foreign_version.event.titleize)
    end
  end

  describe 'GET /admin/audit_logs/:id' do
    let!(:version) do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request.controller_info = paper_trail_info_for(admin)
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
      end
      PaperTrail::Version.last
    end

    context 'when authenticated as administrator' do
      before { sign_in(admin) }

      it 'returns success' do
        get admin_audit_log_path(version)
        expect(response).to have_http_status(:success)
      end

      it 'returns HTML content type' do
        get admin_audit_log_path(version)
        expect(response.content_type).to include('text/html')
      end
    end

    context 'when the version belongs to another household' do
      let(:version) { create_audit_version(household: other_household) }

      before { sign_in(admin) }

      it 'returns not found' do
        get admin_audit_log_path(version)
        expect(response).to have_http_status(:not_found)
      end
    end

    context 'when authenticated as non-administrator' do
      before { sign_in(regular_user) }

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

  def insert_unlisted_audit_record
    PaperTrail::Version.connection.execute(
      PaperTrail::Version.sanitize_sql_array(
        [
          'INSERT INTO versions (household_id, item_type, item_id, event, object, created_at) ' \
          'VALUES (?, ?, ?, ?, ?, ?)',
          current_request_household.id,
          'UnlistedAuditRecord',
          123,
          'custom/audit_event',
          { changed: true }.to_json,
          Time.current.to_fs(:db)
        ]
      )
    )
  end

  def create_audit_version(household:, item_type: 'User', event: 'update')
    version_id = PaperTrail::Version.connection.select_value(
      PaperTrail::Version.sanitize_sql_array(
        [
          'INSERT INTO versions (household_id, item_type, item_id, event, created_at) ' \
          'VALUES (?, ?, ?, ?, ?) RETURNING id',
          household.id,
          item_type,
          users(:jane).id,
          event,
          Time.current.to_fs(:db)
        ]
      )
    )
    PaperTrail::Version.find(version_id)
  end

  def current_request_household
    Household.find_by!(slug: default_request_household_slug)
  end

  def paper_trail_info_for(user)
    household = ensure_api_household_for(user)
    account = Account.find_by(email: user.email_address)
    membership = household.household_memberships.find_by(account: account)
    { household_id: household.id, actor_membership_id: membership&.id }.compact
  end

  def other_household
    Household.find_or_create_by!(slug: 'other-audit-household') do |household|
      household.name = 'Other Audit Household'
      household.timezone = Time.zone.name
    end
  end
end
