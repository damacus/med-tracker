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

    # Travel to a time well past the cooldown window so fixture takes don't interfere
    before { travel_to Time.current.end_of_day - 1.minute }

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
    let(:person_medication) { person_medications(:jane_vitamin_d) }

    context 'when everything is valid and no override amount' do
      let(:result) { call_service(source: person_medication) }

      it_behaves_like 'a successful dose', 1000 # jane_vitamin_d dose_amount
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

    before { travel_to Time.current.end_of_day - 1.minute }

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

  describe 'per-dose inventory tracking' do
    let(:location) { locations(:home) }
    let(:inventory_medication) do
      create(
        :medication,
        name: 'Pregnacare Plus tablets and capsules (Vitabiotics Ltd)',
        location: location,
        dosage_amount: nil,
        dosage_unit: nil,
        current_supply: 84,
        supply_at_last_restock: 84,
        reorder_threshold: 21
      )
    end
    let!(:tablet_option) do
      create(
        :dosage,
        medication: inventory_medication,
        amount: 1,
        unit: 'tablet',
        frequency: 'As directed',
        current_supply: 56,
        reorder_threshold: 14
      )
    end
    let!(:capsule_option) do
      create(
        :dosage,
        medication: inventory_medication,
        amount: 1,
        unit: 'capsule',
        frequency: 'As directed',
        current_supply: 28,
        reorder_threshold: 7
      )
    end

    def build_combo_person_medication(medication)
      create(
        :person_medication,
        person: people(:john),
        medication: medication,
        dose_amount: 1,
        dose_unit: 'tablet',
        max_daily_doses: 1,
        min_hours_between_doses: 24,
        dose_cycle: :daily
      )
    end

    it 'decrements only the matching dose-option inventory and keeps aggregate stock in sync' do
      person_medication = build_combo_person_medication(inventory_medication)
      result = nil

      expect do
        result = call_service(source: person_medication)
        expect(result.success).to be(true)
      end.to change(MedicationTake, :count).by(1)

      expect(tablet_option.reload.current_supply).to eq(55)
      expect(capsule_option.reload.current_supply).to eq(28)
      expect(inventory_medication.reload.current_supply).to eq(83)
    end
  end

  describe 'result object' do
    let(:schedule) { schedules(:john_paracetamol) }

    before { travel_to Time.current.end_of_day - 1.minute }

    it 'exposes success, take, and error attributes' do
      result = call_service(source: schedule)
      expect(result).to respond_to(:success, :take, :error)
    end

    it 'is immutable' do
      result = call_service(source: schedule)
      expect { result.success = false }.to raise_error(NoMethodError)
    end
  end

  describe 'dose_taken.med_tracker' do
    def captured_event_payloads(event_name, &)
      payloads = []
      subscriber = lambda do |*args|
        payloads << ActiveSupport::Notifications::Event.new(*args).payload
      end

      ActiveSupport::Notifications.subscribed(subscriber, event_name, &)

      payloads
    end

    def expect_dose_taken_payload(payloads, result:, source:, amount_ml:, source_type:)
      expect(payloads).to contain_exactly(
        include(
          take_id: result.take.id,
          source_type: source_type,
          source_id: source.id,
          person_id: source.person_id,
          medication_id: source.medication_id,
          inventory_medication_id: source.medication_id,
          inventory_location_id: source.medication.location_id,
          amount_ml: amount_ml,
          taken_at: result.take.taken_at
        )
      )
    end

    it 'publishes one event for a successful scheduled dose' do
      schedule = schedules(:john_paracetamol)
      result = nil

      travel_to Time.current.end_of_day - 1.minute do
        payloads = captured_event_payloads('dose_taken.med_tracker') do
          result = call_service(source: schedule, amount_override: '750')
        end

        expect_dose_taken_payload(
          payloads,
          result: result,
          source: schedule,
          amount_ml: 750.0,
          source_type: 'schedule'
        )
      end
    end

    it 'publishes one event for a successful as-needed dose' do
      person_medication = person_medications(:jane_vitamin_d)
      result = nil

      payloads = captured_event_payloads('dose_taken.med_tracker') do
        result = call_service(source: person_medication)
      end

      expect_dose_taken_payload(
        payloads,
        result: result,
        source: person_medication,
        amount_ml: 1000.0,
        source_type: 'person_medication'
      )
    end

    it 'does not publish when dose creation fails because stock is unavailable' do
      schedule = schedules(:john_paracetamol)
      schedule.medication.update!(current_supply: 0)

      payloads = captured_event_payloads('dose_taken.med_tracker') do
        call_service(source: schedule)
      end

      expect(payloads).to be_empty
    end

    it 'does not publish when timing restrictions block the dose' do
      schedule = schedules(:john_paracetamol)

      travel_to Time.current.end_of_day - 1.minute do
        schedule.medication_takes.create!(
          taken_at: 1.minute.ago,
          amount_ml: schedule.default_dose_amount,
          taken_from_medication: schedule.medication,
          taken_from_location: schedule.medication.location
        )

        payloads = captured_event_payloads('dose_taken.med_tracker') do
          call_service(source: schedule)
        end

        expect(payloads).to be_empty
      end
    end

    it 'does not publish when the amount is invalid' do
      schedule = schedules(:john_paracetamol)

      payloads = captured_event_payloads('dose_taken.med_tracker') do
        call_service(source: schedule, amount_override: '0')
      end

      expect(payloads).to be_empty
    end

    it 'does not publish when stock-source selection is required' do
      schedule = schedules(:john_paracetamol)
      create_matching_medication(
        medication: schedule.medication,
        location: Location.create!(name: 'Selection Required Home')
      )

      payloads = captured_event_payloads('dose_taken.med_tracker') do
        call_service(source: schedule)
      end

      expect(payloads).to be_empty
    end

    it 'does not publish when the selected inventory source is invalid' do
      schedule = schedules(:john_paracetamol)

      payloads = captured_event_payloads('dose_taken.med_tracker') do
        call_service(source: schedule, taken_from_medication_id: -1)
      end

      expect(payloads).to be_empty
    end
  end

  def create_matching_medication(medication:, location:)
    Medication.create!(
      name: medication.name,
      location: location,
      dosage_amount: medication.dosage_amount,
      dosage_unit: medication.dosage_unit,
      current_supply: 12,
      reorder_threshold: 2
    )
  end
end
