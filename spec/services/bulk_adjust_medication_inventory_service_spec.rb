# frozen_string_literal: true

require 'rails_helper'

RSpec.describe BulkAdjustMedicationInventoryService do
  subject(:service) { described_class.new }

  let!(:paracetamol) { create(:medication, current_supply: 80) }
  let!(:aspirin) { create(:medication, current_supply: 25) }

  describe '#call' do
    it 'updates every medication in one batch' do
      result = service.call(
        medications: [paracetamol, aspirin],
        adjustments: {
          paracetamol.id.to_s => '74',
          aspirin.id.to_s => '0'
        },
        reason: 'House stock check'
      )

      expect(result).to be_success
      expect(paracetamol.reload.current_supply).to eq(74)
      expect(aspirin.reload.current_supply).to eq(0)
      expect(paracetamol.paper_trail_event).to include('reason: House stock check')
      expect(aspirin.paper_trail_event).to include('reason: House stock check')
    end

    it 'rolls back the whole batch when one quantity is invalid' do
      result = service.call(
        medications: [paracetamol, aspirin],
        adjustments: {
          paracetamol.id.to_s => '74',
          aspirin.id.to_s => '-1'
        },
        reason: 'House stock check'
      )

      expect(result).not_to be_success
      expect(result.error).to eq('Quantity cannot be negative')
      expect(paracetamol.reload.current_supply).to eq(80)
      expect(aspirin.reload.current_supply).to eq(25)
    end

    it 'rejects an empty batch' do
      result = service.call(medications: [], adjustments: {}, reason: 'House stock check')

      expect(result).not_to be_success
      expect(result.error).to eq('Select at least one medicine')
    end
  end
end
