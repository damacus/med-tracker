# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RestockMedicationService do
  describe '#call' do
    let(:medication) do
      create(
        :medication,
        current_supply: 10,
        supply_at_last_restock: 10,
        reorder_status: :ordered,
        ordered_at: 1.day.ago,
        reordered_at: 1.hour.ago
      )
    end

    it 'restocks the medication and records the audit event' do
      restock_date = Date.current

      result = described_class.new.call(medication: medication, quantity: '12', restock_date: restock_date)

      expect(result).to be_success
      expect(result.error).to be_nil
      expect(medication.reload).to have_attributes(
        current_supply: 22,
        supply_at_last_restock: 22,
        reorder_status: nil,
        ordered_at: nil,
        reordered_at: nil,
        paper_trail_event: "restock (qty: 12, date: #{restock_date.iso8601})"
      )
    end

    it 'restocks with a decimal quantity' do
      medication.update!(dosage_unit: 'ml', current_supply: 100, supply_at_last_restock: 100)
      restock_date = Date.current

      result = described_class.new.call(medication: medication, quantity: '12.5', restock_date: restock_date)

      expect(result).to be_success
      expect(medication.reload).to have_attributes(
        current_supply: BigDecimal('112.5'),
        supply_at_last_restock: BigDecimal('112.5'),
        paper_trail_event: "restock (qty: 12.5, date: #{restock_date.iso8601})"
      )
    end

    it 'returns an error for a non-positive quantity' do
      result = described_class.new.call(medication: medication, quantity: '0', restock_date: Date.current)

      expect(result).not_to be_success
      expect(result.error).to eq('Quantity must be greater than 0')
      expect(medication.reload.current_supply).to eq(10)
    end

    it 'returns an error for a missing restock date' do
      result = described_class.new.call(medication: medication, quantity: '12', restock_date: nil)

      expect(result).not_to be_success
      expect(result.error).to eq('Restock date is invalid')
      expect(medication.reload.current_supply).to eq(10)
    end

    it 'returns validation errors raised by the model command' do
      allow(medication).to receive(:restock!).and_raise(ActiveRecord::RecordInvalid.new(medication))
      medication.errors.add(:current_supply, 'is invalid')

      result = described_class.new.call(medication: medication, quantity: '12', restock_date: Date.current)

      expect(result).not_to be_success
      expect(result.error).to eq('Current supply is invalid')
    end
  end
end
