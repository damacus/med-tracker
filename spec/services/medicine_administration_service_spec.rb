# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicineAdministrationService do
  fixtures :accounts, :people, :medicines, :dosages, :prescriptions, :person_medicines, :medication_takes

  describe '#call' do
    context 'with a prescription' do
      let(:prescription) { prescriptions(:john_paracetamol) }

      it 'creates a medication take when administration is allowed' do
        result = described_class.call(takeable: prescription)

        expect(result).to be_success
        expect(result.medication_take).to be_a(MedicationTake)
        expect(result.medication_take).to be_persisted
        expect(result.medication_take.prescription).to eq(prescription)
        expect(result.medication_take.taken_at).to be_within(1.second).of(Time.current)
      end

      it 'uses the dosage amount when no amount_ml is provided' do
        result = described_class.call(takeable: prescription)

        expect(result.medication_take.amount_ml).to eq(prescription.dosage.amount)
      end

      it 'uses the provided amount_ml when given' do
        result = described_class.call(takeable: prescription, amount_ml: 250)

        expect(result.medication_take.amount_ml).to eq(250)
      end

      it 'returns failure when out of stock' do
        prescription.medicine.update!(stock: 0)

        result = described_class.call(takeable: prescription)

        expect(result).to be_failure
        expect(result.error).to eq(:out_of_stock)
        expect(result.message).to include('out of stock')
      end

      it 'returns failure when timing restrictions are not met' do
        prescription.medication_takes.create!(taken_at: Time.current, amount_ml: 500)
        prescription.medication_takes.create!(taken_at: Time.current, amount_ml: 500)
        prescription.medication_takes.create!(taken_at: Time.current, amount_ml: 500)
        prescription.medication_takes.create!(taken_at: Time.current, amount_ml: 500)

        result = described_class.call(takeable: prescription)

        expect(result).to be_failure
        expect(result.error).to eq(:timing_restriction)
        expect(result.message).to include('timing restrictions')
      end
    end

    context 'with a person medicine' do
      let(:medicine) { create(:medicine, stock: 100) }
      let(:person_medicine) { create(:person_medicine, medicine: medicine) }

      it 'creates a medication take when administration is allowed' do
        result = described_class.call(takeable: person_medicine)

        expect(result).to be_success
        expect(result.medication_take).to be_a(MedicationTake)
        expect(result.medication_take).to be_persisted
        expect(result.medication_take.person_medicine).to eq(person_medicine)
      end

      it 'uses the medicine dosage_amount when no amount_ml is provided' do
        result = described_class.call(takeable: person_medicine)

        expect(result.medication_take.amount_ml).to eq(person_medicine.medicine.dosage_amount)
      end

      it 'uses the provided amount_ml when given' do
        result = described_class.call(takeable: person_medicine, amount_ml: 100)

        expect(result.medication_take.amount_ml).to eq(100)
      end

      it 'returns failure when out of stock' do
        medicine.update!(stock: 0)

        result = described_class.call(takeable: person_medicine)

        expect(result).to be_failure
        expect(result.error).to eq(:out_of_stock)
      end

      it 'returns failure when timing restrictions are not met' do
        restricted = create(:person_medicine, :with_max_doses, medicine: medicine, max_daily_doses: 1)
        restricted.medication_takes.create!(taken_at: Time.current, amount_ml: 500)

        result = described_class.call(takeable: restricted)

        expect(result).to be_failure
        expect(result.error).to eq(:timing_restriction)
      end
    end
  end

  describe 'Result' do
    it 'success result responds to success? and failure?' do
      result = described_class::Result.new(success: true, medication_take: MedicationTake.new)

      expect(result).to be_success
      expect(result).not_to be_failure
    end

    it 'failure result responds to success? and failure?' do
      result = described_class::Result.new(success: false, error: :out_of_stock, message: 'Out of stock')

      expect(result).not_to be_success
      expect(result).to be_failure
    end
  end
end
