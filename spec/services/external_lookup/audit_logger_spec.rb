# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ExternalLookup::AuditLogger do
  subject(:audit_logger) { described_class.new }

  let(:user_id) { 42 }

  before do
    PaperTrail.request.whodunnit = user_id
    PaperTrail.request.controller_info = { ip: '10.0.0.1', request_id: 'req-abc-123' }
  end

  after do
    PaperTrail.request.whodunnit = nil
    PaperTrail.request.controller_info = {}
  end

  describe '#record' do
    it 'creates a PaperTrail::Version with item_type ExternalLookup' do
      expect do
        audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin',
                            result_status: 'success', result_count: 3)
      end.to change { PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').count }.by(1)
    end

    it 'persists the event name as source/event and the request context' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin',
                          result_status: 'success', result_count: 3)
      version = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last
      expect(version.event).to eq('nhs_dmd/search')
      expect(version.whodunnit).to eq(user_id.to_s)
      expect(version.ip).to eq('10.0.0.1')
      expect(version.request_id).to eq('req-abc-123')
    end

    it 'partitions manual audit versions by household context' do
      household = Household.create!(name: 'Lookup Audit Household')
      account = Account.create!(email: 'lookup-audit@example.test', status: :verified)
      membership = household.household_memberships.create!(account: account, role: :owner, status: :active)
      PaperTrail.request.controller_info = {
        ip: '10.0.0.1',
        request_id: 'req-abc-123',
        household_id: household.id,
        actor_membership_id: membership.id
      }

      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin', result_status: 'success')

      version = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last
      expect(version.household_id).to eq(household.id)
      expect(version.actor_membership_id).to eq(membership.id)
    end

    it 'stores the readable query, SHA256 query hash, and metadata in object JSON' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin 300mg',
                          result_status: 'success', result_count: 3)
      version = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last
      data = JSON.parse(version.object)
      expect(data['query']).to eq('Aspirin 300mg')
      expect(data['query_hash']).to eq(Digest::SHA256.hexdigest('aspirin 300mg'))
      expect(data['result_status']).to eq('success')
      expect(data['result_count']).to eq(3)
    end

    it 'stores additional lookup details when provided' do
      audit_logger.record(source: 'nhs_website_content', event: 'medicine_guidance_lookup',
                          query: 'Panadol 500mg tablets', result_status: 'success',
                          result_count: 1,
                          metadata: {
                            'matched_title' => 'Paracetamol for adults',
                            'matched_url' => 'https://www.nhs.uk/medicines/paracetamol-for-adults/'
                          })
      version = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last
      data = JSON.parse(version.object)

      expect(data['matched_title']).to eq('Paracetamol for adults')
      expect(data['matched_url']).to eq('https://www.nhs.uk/medicines/paracetamol-for-adults/')
    end

    it 'normalises the query before hashing (lowercase, stripped)' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: '  Aspirin  ', result_status: 'success')
      v1 = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last

      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'aspirin', result_status: 'success')
      v2 = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last

      expect(JSON.parse(v1.object)['query_hash']).to eq(JSON.parse(v2.object)['query_hash'])
    end

    it 'defaults result_count to 0' do
      audit_logger.record(source: 'open_food_facts', event: 'barcode_lookup',
                          query: '12345', result_status: 'not_found')
      version = PaperTrail::Version.where(item_type: 'ExternalMedicineLookup').last
      expect(JSON.parse(version.object)['result_count']).to eq(0)
    end

    it 'does not raise if PaperTrail request context is not set' do
      PaperTrail.request.whodunnit = nil
      PaperTrail.request.controller_info = {}

      expect do
        audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'test', result_status: 'success')
      end.not_to raise_error
    end

    it 'silently rescues errors and logs them' do
      allow(PaperTrail::Version).to receive(:insert).and_raise(ActiveRecord::StatementInvalid)
      allow(Rails.logger).to receive(:error)

      expect do
        audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'test', result_status: 'success')
      end.not_to raise_error

      expect(Rails.logger).to have_received(:error).with(/ExternalLookup::AuditLogger failed/)
    end
  end
end
