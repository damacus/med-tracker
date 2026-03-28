# frozen_string_literal: true

require 'rails_helper'

RSpec.describe TakeMedicationService do
  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :dosages, :schedules, :person_medications

  subject(:service) { described_class.new }

  let(:user) { users(:john) }

  # Shared helper to invoke the service
  def call_service(source:, amount_override: nil, taken_from_medication_id: nil)
    service.call(
      source: source,
      amount_override: amount_override,
      taken_from_medication_id: taken_from_medication_id,
      user: user
    )
  end

  shared_examples 'a successful dose' do |expected_amount|
    it 'returns success' do
      expect(result.success).to be true
    end

    it 'returns a persisted MedicationTake' do
      expect(result.take).to be_a(MedicationTake)
      expect(result.take).to be_persisted
    end

    it 'records the correct amount' do
      expect(result.take.amount_ml).to eq(BigDecimal(expected_amount.to_s))
    end

    it 'records no error' do
      expect(result.error).to be_nil
    end
  end

  describe '#call with a Schedule source' do
    let(:schedule) { schedules(:john_paracetamol) }

    context 'when everything is valid and no override amount' do
      let(:result) { call_service(source: schedule) }

      it_behaves_like 'a successful dose', 1000 # paracetamol_adult dosage amount
    end

    context 'when an explicit amount override is provided' do
      let(:result) { call_service(source: schedule, amount_override: '750') }

      it_behaves_like 'a successful dose', 750
    end

    context 'when the medication is out of stock' do
      before { schedule.medication.update!(current_supply: 0) }

      it 'returns :out_of_stock error' do
        expect(call_service(source: schedule).error).to eq(:out_of_stock)
      end

      it 'does not create a MedicationTake' do
        expect { call_service(source: schedule) }.not_to change(MedicationTake, :count)
      end
    end

    context 'when timing restrictions prevent the dose' do
      before do
        # Create a recent take to trigger the cooldown
        schedule.medication_takes.create!(
          taken_at: 1.minute.ago,
          amount_ml: schedule.default_dose_amount,
          taken_from_medication: schedule.medication,
          taken_from_location: schedule.medication.location
        )
      end

      it 'returns :cooldown error' do
        expect(call_service(source: schedule).error).to eq(:cooldown)
      end

      it 'does not create a MedicationTake' do
        expect { call_service(source: schedule) }.not_to change(MedicationTake, :count)
      end
    end

    context 'when the resolved dose amount is nil' do
      before { allow(schedule).to receive(:default_dose_amount).and_return(nil) }

      it 'returns :invalid_amount error' do
        expect(call_service(source: schedule).error).to eq(:invalid_amount)
      end
    end

    context 'when the override amount is zero' do
      it 'returns :invalid_amount error' do
        expect(call_service(source: schedule, amount_override: '0').error).to eq(:invalid_amount)
      end
    end

    context 'when the override amount is negative' do
      it 'returns :invalid_amount error' do
        expect(call_service(source: schedule, amount_override: '-5').error).to eq(:invalid_amount)
      end
    end

    context 'when the override amount is not a number' do
      it 'returns :invalid_amount error' do
        expect(call_service(source: schedule, amount_override: 'abc').error).to eq(:invalid_amount)
      end
    end
  end

  describe '#call with a PersonMedication source' do
    let(:person_medication) { person_medications(:john_vitamin_d) }

    context 'when everything is valid and no override amount' do
      let(:result) { call_service(source: person_medication) }

      it_behaves_like 'a successful dose', 1000 # john_vitamin_d dose_amount
    end

    context 'when an explicit amount override is provided' do
      let(:result) { call_service(source: person_medication, amount_override: '500') }

      it_behaves_like 'a successful dose', 500
    end

    context 'when the medication is out of stock' do
      before { person_medication.medication.update!(current_supply: 0) }

      it 'returns :out_of_stock error' do
        expect(call_service(source: person_medication).error).to eq(:out_of_stock)
      end

      it 'does not create a MedicationTake' do
        expect { call_service(source: person_medication) }.not_to change(MedicationTake, :count)
      end
    end
  end

  describe 'taken_at override' do
    let(:schedule) { schedules(:john_paracetamol) }
    let(:custom_time) { 2.hours.ago }

    it 'records the custom taken_at when provided' do
      result = service.call(
        source: schedule,
        amount_override: nil,
        taken_from_medication_id: nil,
        user: user,
        taken_at: custom_time
      )
      expect(result.success).to be true
      expect(result.take.taken_at).to be_within(1.second).of(custom_time)
    end

    it 'defaults taken_at to now when not provided' do
      result = call_service(source: schedule)
      expect(result.take.taken_at).to be_within(5.seconds).of(Time.current)
    end
  end

  describe 'result object' do
    let(:schedule) { schedules(:john_paracetamol) }

    it 'exposes success, take, and error attributes' do
      result = call_service(source: schedule)
      expect(result).to respond_to(:success, :take, :error)
    end

    it 'is immutable' do
      result = call_service(source: schedule)
      expect { result.success = false }.to raise_error(NoMethodError)
    end
  end
end
