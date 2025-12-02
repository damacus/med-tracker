# frozen_string_literal: true

require 'rails_helper'

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

      it 'displays audit trail heading' do
        get admin_audit_logs_path
        expect(response.body).to include('Audit Trail')
      end

      it 'displays filter form with record type and event type dropdowns' do
        get admin_audit_logs_path
        expect(response.body).to include('Record Type')
        expect(response.body).to include('Event Type')
        expect(response.body).to include('item_type')
        expect(response.body).to include('event')
      end

      it 'displays table headers for timestamp, record type, event, user' do
        # Create an audit entry so the table is displayed
        PaperTrail.request.whodunnit = admin.id
        PaperTrail.request(enabled: true) do
          users(:jane).update!(role: :nurse)
        end

        get admin_audit_logs_path
        expect(response.body).to include('Timestamp')
        expect(response.body).to include('Record Type')
        expect(response.body).to include('Event')
        expect(response.body).to include('User')
        expect(response.body).to include('IP Address')
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

  # AUDIT-013: Complete audit trail review workflow
  describe 'AUDIT-013: Complete audit trail review workflow' do
    before { sign_in_as(admin) }

    describe 'filtering' do
      before do
        # Create audit entries for different record types
        PaperTrail.request.whodunnit = admin.id
        PaperTrail.request(enabled: true) do
          users(:jane).update!(role: :nurse)
          people(:john).update!(name: 'John Updated')
        end
      end

      it 'filters by record type' do
        get admin_audit_logs_path, params: { item_type: 'User' }
        expect(response).to have_http_status(:success)
        expect(response.body).to include('User')
      end

      it 'filters by event type' do
        get admin_audit_logs_path, params: { event: 'update' }
        expect(response).to have_http_status(:success)
        expect(response.body).to include('Update')
      end

      it 'shows clear button when filters are active' do
        get admin_audit_logs_path, params: { item_type: 'User' }
        expect(response.body).to include('Clear')
      end
    end

    describe 'detail view' do
      let!(:version) do
        PaperTrail.request.whodunnit = admin.id
        PaperTrail.request(enabled: true) do
          users(:jane).update!(role: :nurse)
        end
        PaperTrail::Version.last
      end

      it 'shows audit log details' do
        get admin_audit_log_path(version)
        expect(response).to have_http_status(:success)
        expect(response.body).to include('Audit Log Details')
        expect(response.body).to include('Event Information')
      end

      it 'displays previous state for update events' do
        get admin_audit_log_path(version)
        expect(response.body).to include('Previous State')
      end

      it 'displays new state for update events' do
        get admin_audit_log_path(version)
        expect(response.body).to include('New State')
      end

      it 'shows back to audit logs link' do
        get admin_audit_log_path(version)
        expect(response.body).to include('Back to Audit Logs')
      end
    end

    describe 'pagination' do
      before do
        # Create more than 50 audit entries to trigger pagination.
        # NOTE: We create PaperTrail::Version records directly here for performance.
        # Creating 55 actual model changes would be slow and unnecessary since we're
        # testing pagination UI, not the audit trail generation mechanism itself.
        PaperTrail.request.whodunnit = admin.id
        55.times do |i|
          PaperTrail::Version.create!(
            item_type: 'User',
            item_id: i + 1000,
            event: 'update',
            whodunnit: admin.id.to_s,
            created_at: Time.current
          )
        end
      end

      it 'shows pagination controls when there are many entries' do
        get admin_audit_logs_path
        expect(response.body).to include('Pagination')
        expect(response.body).to include('Showing')
        expect(response.body).to include('results')
      end

      it 'shows next page link' do
        get admin_audit_logs_path
        expect(response.body).to include('Next')
      end

      it 'navigates to page 2' do
        get admin_audit_logs_path, params: { page: 2 }
        expect(response).to have_http_status(:success)
      end
    end
  end

  # AUDIT-014: Audit trail for medication take lifecycle
  describe 'AUDIT-014: Medication take lifecycle audit' do
    before { sign_in_as(admin) }

    describe 'medication take creation logging' do
      before do
        PaperTrail.request.whodunnit = carer.id
        PaperTrail.request(enabled: true) do
          MedicationTake.create!(
            prescription: prescription,
            taken_at: Time.current
          )
        end
      end

      it 'logs medication take creation' do
        get admin_audit_logs_path, params: { item_type: 'MedicationTake' }
        expect(response).to have_http_status(:success)
        expect(response.body).to include('Medication Take')
        expect(response.body).to include('Create')
      end

      it 'shows whodunnit (carer) in audit log' do
        version = PaperTrail::Version.where(item_type: 'MedicationTake').last
        get admin_audit_log_path(version)
        expect(response.body).to include(carer.name)
      end

      it 'shows prescription_id in new state' do
        version = PaperTrail::Version.where(item_type: 'MedicationTake').last
        get admin_audit_log_path(version)
        expect(response.body).to include('New State')
        expect(response.body).to include('prescription_id')
      end

      it 'shows taken_at timestamp in new state' do
        version = PaperTrail::Version.where(item_type: 'MedicationTake').last
        get admin_audit_log_path(version)
        expect(response.body).to include('taken_at')
      end
    end

    describe 'IP address recording' do
      it 'displays IP address in audit log list' do
        # Create an actual MedicationTake with IP tracking enabled
        # This tests the end-to-end audit trail functionality
        PaperTrail.request.whodunnit = carer.id
        PaperTrail.request.controller_info = { ip: '192.168.1.100' }
        PaperTrail.request(enabled: true) do
          MedicationTake.create!(
            prescription: prescription,
            taken_at: Time.current
          )
        end

        get admin_audit_logs_path, params: { item_type: 'MedicationTake' }
        expect(response.body).to include('192.168.1.100')
      end
    end
  end
end
