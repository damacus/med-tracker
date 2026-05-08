# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ExternalLookupAuditEvent do
  describe 'validations' do
    subject(:event) do
      described_class.new(
        source: 'nhs_dmd',
        event: 'search',
        result_status: 'success',
        result_count: 1
      )
    end

    it { is_expected.to be_valid }

    it 'validates source is present' do
      event.source = nil
      expect(event).not_to be_valid
      expect(event.errors[:source]).to be_present
    end

    it 'validates source is a known value' do
      event.source = 'unknown_source'
      expect(event).not_to be_valid
      expect(event.errors[:source]).to be_present
    end

    it 'validates event is present' do
      event.event = nil
      expect(event).not_to be_valid
      expect(event.errors[:event]).to be_present
    end

    it 'validates event is a known value' do
      event.event = 'unknown_event'
      expect(event).not_to be_valid
      expect(event.errors[:event]).to be_present
    end

    it 'validates result_status is present' do
      event.result_status = nil
      expect(event).not_to be_valid
      expect(event.errors[:result_status]).to be_present
    end

    it 'validates result_status is a known value' do
      event.result_status = 'unknown_status'
      expect(event).not_to be_valid
      expect(event.errors[:result_status]).to be_present
    end

    it 'validates result_count is a non-negative integer' do
      event.result_count = -1
      expect(event).not_to be_valid
      expect(event.errors[:result_count]).to be_present
    end

    ExternalLookupAuditEvent::SOURCES.each do |source|
      it "accepts source '#{source}'" do
        event.source = source
        expect(event).to be_valid
      end
    end

    ExternalLookupAuditEvent::EVENTS.each do |evt|
      it "accepts event '#{evt}'" do
        event.event = evt
        expect(event).to be_valid
      end
    end

    ExternalLookupAuditEvent::RESULT_STATUSES.each do |status|
      it "accepts result_status '#{status}'" do
        event.result_status = status
        expect(event).to be_valid
      end
    end
  end
end
