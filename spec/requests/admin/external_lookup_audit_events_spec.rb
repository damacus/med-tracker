# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::ExternalLookupAuditEvents' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }

  describe 'GET /admin/external_lookup_audit_events' do
    context 'when authenticated as administrator' do
      before { sign_in(admin) }

      it 'returns success' do
        get admin_external_lookup_audit_events_path
        expect(response).to have_http_status(:success)
      end

      it 'returns HTML content type' do
        get admin_external_lookup_audit_events_path
        expect(response.content_type).to include('text/html')
      end
    end

    context 'when authenticated as non-administrator' do
      before { sign_in(regular_user) }

      it 'denies access' do
        get admin_external_lookup_audit_events_path
        expect(response).to redirect_to(root_path)
      end
    end

    context 'when not authenticated' do
      it 'redirects to login' do
        get admin_external_lookup_audit_events_path
        expect(response).to redirect_to(login_path)
      end
    end
  end

  describe 'GET /admin/external_lookup_audit_events with filters' do
    before do
      sign_in(admin)
      ExternalLookupAuditEvent.create!(source: 'nhs_dmd', event: 'search', result_status: 'success', result_count: 2)
      ExternalLookupAuditEvent.create!(source: 'open_food_facts', event: 'barcode_lookup',
                                       result_status: 'not_found', result_count: 0)
    end

    it 'accepts source filter parameter' do
      get admin_external_lookup_audit_events_path, params: { source: 'nhs_dmd' }
      expect(response).to have_http_status(:success)
    end

    it 'accepts result_status filter parameter' do
      get admin_external_lookup_audit_events_path, params: { result_status: 'success' }
      expect(response).to have_http_status(:success)
    end

    it 'filters by source' do
      get admin_external_lookup_audit_events_path, params: { source: 'nhs_dmd' }
      expect(response.body).to include('Nhs Dmd')
    end

    it 'filters by result_status' do
      get admin_external_lookup_audit_events_path, params: { result_status: 'not_found' }
      expect(response.body).to include('Not Found')
    end
  end
end
