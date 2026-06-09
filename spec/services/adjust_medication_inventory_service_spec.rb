# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdjustMedicationInventoryService do
  subject(:service) { described_class.new }

  let(:medication) { create(:medication, current_supply: 50) }

  describe '#call' do
    context 'when new_quantity is valid' do
      it 'returns a successful result' do
        result = service.call(medication: medication, new_quantity: '30')
        expect(result).to be_success
        expect(result.error).to be_nil
      end

      it 'updates current_supply to the new quantity' do
        service.call(medication: medication, new_quantity: '30')
        expect(medication.reload.current_supply).to eq(BigDecimal('30'))
      end

      it 'accepts decimal quantities' do
        service.call(medication: medication, new_quantity: '12.5')
        expect(medication.reload.current_supply).to eq(BigDecimal('12.5'))
      end

      it 'returns the medication in the result' do
        result = service.call(medication: medication, new_quantity: '30')
        expect(result.medication).to eq(medication)
      end

      it 'sets the paper_trail_event without a reason' do
        service.call(medication: medication, new_quantity: '30')
        expect(medication.paper_trail_event).to eq('adjust inventory (qty: 30)')
      end

      it 'includes reason in the paper_trail_event when provided' do
        service.call(medication: medication, new_quantity: '30', reason: 'counted manually')
        expect(medication.paper_trail_event).to eq('adjust inventory (qty: 30, reason: counted manually)')
      end

      it 'accepts zero as a valid quantity (setting inventory to 0)' do
        result = service.call(medication: medication, new_quantity: '0')
        expect(result).to be_success
        expect(medication.reload.current_supply).to eq(BigDecimal('0'))
      end
    end

    context 'when new_quantity is invalid' do
      it 'returns a failure result for non-numeric input' do
        result = service.call(medication: medication, new_quantity: 'abc')
        expect(result).not_to be_success
        expect(result.error).to eq('Quantity must be a valid number')
      end

      it 'does not modify the medication when quantity is non-numeric' do
        service.call(medication: medication, new_quantity: 'abc')
        expect(medication.reload.current_supply).to eq(50)
      end

      it 'returns a failure result for negative quantity' do
        result = service.call(medication: medication, new_quantity: '-5')
        expect(result).not_to be_success
        expect(result.error).to eq('Quantity cannot be negative')
      end

      it 'does not modify the medication when quantity is negative' do
        service.call(medication: medication, new_quantity: '-5')
        expect(medication.reload.current_supply).to eq(50)
      end

      it 'returns a failure result for blank string' do
        result = service.call(medication: medication, new_quantity: '')
        expect(result).not_to be_success
      end
    end

    context 'when ActiveRecord raises RecordInvalid' do
      before do
        allow(medication).to receive(:update!).and_raise(
          ActiveRecord::RecordInvalid.new(medication)
        )
        medication.errors.add(:current_supply, 'is too large')
      end

      it 'returns a failure result with validation message' do
        result = service.call(medication: medication, new_quantity: '30')
        expect(result).not_to be_success
        expect(result.error).to eq('Current supply is too large')
      end
    end

    context 'result object' do
      it 'exposes success?, medication, and error' do
        result = service.call(medication: medication, new_quantity: '30')
        expect(result).to respond_to(:success?, :medication, :error)
      end

      it 'is immutable (Data.define)' do
        result = service.call(medication: medication, new_quantity: '30')
        expect { result.success = false }.to raise_error(NoMethodError)
      end
    end
  end
end
