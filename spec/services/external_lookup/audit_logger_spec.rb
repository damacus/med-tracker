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
    it 'creates an ExternalLookupAuditEvent' do
      expect do
        audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin',
                            result_status: 'success', result_count: 3)
      end.to change(ExternalLookupAuditEvent, :count).by(1)
    end

    it 'persists the correct source, event, and status' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin',
                          result_status: 'success', result_count: 3)
      event = ExternalLookupAuditEvent.last
      expect(event.source).to eq('nhs_dmd')
      expect(event.event).to eq('search')
      expect(event.result_status).to eq('success')
      expect(event.result_count).to eq(3)
    end

    it 'persists the request context from PaperTrail' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin', result_status: 'success')
      event = ExternalLookupAuditEvent.last
      expect(event.whodunnit).to eq(user_id.to_s)
      expect(event.ip).to eq('10.0.0.1')
      expect(event.request_id).to eq('req-abc-123')
    end

    it 'stores a SHA256 hash of the query, not the raw query' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'Aspirin 300mg', result_status: 'success')

      event = ExternalLookupAuditEvent.last
      expected_hash = Digest::SHA256.hexdigest('aspirin 300mg')
      expect(event.query_hash).to eq(expected_hash)
      expect(event.query_hash).not_to include('Aspirin')
    end

    it 'normalises the query before hashing (lowercase, stripped)' do
      audit_logger.record(source: 'nhs_dmd', event: 'search', query: '  Aspirin  ', result_status: 'success')
      event1 = ExternalLookupAuditEvent.last

      audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'aspirin', result_status: 'success')
      event2 = ExternalLookupAuditEvent.last

      expect(event1.query_hash).to eq(event2.query_hash)
    end

    it 'defaults result_count to 0' do
      audit_logger.record(
        source: 'open_food_facts', event: 'barcode_lookup', query: '12345', result_status: 'not_found'
      )

      expect(ExternalLookupAuditEvent.last.result_count).to eq(0)
    end

    it 'does not raise if PaperTrail request context is not set' do
      PaperTrail.request.whodunnit = nil
      PaperTrail.request.controller_info = {}

      expect do
        audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'test', result_status: 'success')
      end.not_to raise_error
    end

    it 'silently rescues validation errors and logs them' do
      allow(ExternalLookupAuditEvent).to receive(:create!).and_raise(ActiveRecord::RecordInvalid)
      allow(Rails.logger).to receive(:error)

      expect do
        audit_logger.record(source: 'nhs_dmd', event: 'search', query: 'test', result_status: 'success')
      end.not_to raise_error

      expect(Rails.logger).to have_received(:error).with(/ExternalLookup::AuditLogger failed/)
    end
  end
end
