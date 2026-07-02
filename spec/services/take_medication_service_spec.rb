# frozen_string_literal: true

require 'rails_helper'

RSpec.describe TakeMedicationService do
  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :dosages, :schedules, :person_medications

  subject(:service) { described_class.new }

  let(:user) { users(:john) }

  before do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
  end

  # Shared helper to invoke the service
  def call_service(source:, amount_override: nil, taken_from_medication_id: nil, **)
    service.call(
      source: source,
      amount_override: amount_override,
      taken_from_medication_id: taken_from_medication_id,
      user: user,
      **
    )
  end

  def captured_event_payloads(event_name, &)
    payloads = []
    subscriber = lambda do |*args|
      payloads << ActiveSupport::Notifications::Event.new(*args).payload
    end

    ActiveSupport::Notifications.subscribed(subscriber, event_name, &)

    payloads
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
      expect(result.take.dose_amount).to eq(BigDecimal(expected_amount.to_s))
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

    context 'when a tapering schedule has an effective amount for today' do
      before do
        schedule.update!(
          schedule_type: :tapering,
          schedule_config: {
            'taper_steps' => [
              {
                'start_date' => Time.zone.today.iso8601,
                'end_date' => Time.zone.today.iso8601,
                'amount' => '500',
                'unit' => schedule.dose_unit,
                'max_daily_doses' => 4,
                'min_hours_between_doses' => 4
              }
            ]
          }
        )
      end

      let(:result) { call_service(source: schedule) }

      it_behaves_like 'a successful dose', 500
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

    context 'when the schedule is paused' do
      before { schedule.pause! }

      it 'returns :paused error' do
        expect(call_service(source: schedule).error).to eq(:paused)
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
          dose_amount: schedule.default_dose_amount,
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

    context 'when another active prescription for the same medication reaches its limit' do
      let(:person) { user.person }
      let(:medication) do
        create(:medication, household: person.household, location: locations(:home), name: 'Overlap Test Medicine')
      end
      let(:schedule) do
        create(
          :schedule,
          person: person,
          medication: medication,
          dosage: nil,
          dose_amount: 500,
          dose_unit: 'mg',
          max_daily_doses: 4,
          min_hours_between_doses: nil
        )
      end
      let(:other_schedule) do
        create(
          :schedule,
          person: person,
          medication: medication,
          dosage: nil,
          dose_amount: 500,
          dose_unit: 'mg',
          max_daily_doses: 1,
          min_hours_between_doses: nil
        )
      end

      before do
        other_schedule.medication_takes.create!(
          taken_at: 1.hour.ago,
          dose_amount: 500,
          dose_unit: 'mg',
          taken_from_medication: medication,
          taken_from_location: medication.location
        )
      end

      it 'does not create a MedicationTake' do
        expect { call_service(source: schedule) }.not_to change(MedicationTake, :count)
      end

      it 'returns an overlapping prescription restriction error' do
        expect(call_service(source: schedule).error).to eq(:overlapping_prescription_restriction)
      end

      it 'publishes the related prescription context with the blocked metric' do
        payloads = captured_event_payloads('take_blocked_by_rules.med_tracker') do
          call_service(source: schedule)
        end

        expect(payloads).to contain_exactly(
          include(
            error: 'overlapping_prescription_restriction',
            decision_source_count: 2,
            decision_blocking_source_type: 'schedule',
            decision_blocking_source_id: other_schedule.id
          )
        )
      end
    end

    context 'when the resolved dose amount is nil' do
      before { allow(schedule).to receive(:effective_dose_amount).and_return(nil) }

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
        dose_amount: nil,
        dose_unit: nil,
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

    def build_effective_capsule_schedule(medication)
      create(
        :schedule,
        person: people(:john),
        medication: medication,
        dosage: nil,
        source_dosage_option: nil,
        dose_amount: 2,
        dose_unit: 'tablet',
        max_daily_doses: 1,
        min_hours_between_doses: 24,
        schedule_type: :tapering,
        schedule_config: effective_capsule_schedule_config
      )
    end

    def effective_capsule_schedule_config
      {
        'taper_steps' => [
          taper_step_config(date: Time.zone.today, unit: 'capsule')
        ]
      }
    end

    def two_step_taper_schedule_config(capsule_date:, tablet_date:)
      {
        'taper_steps' => [
          taper_step_config(date: capsule_date, unit: 'capsule'),
          taper_step_config(date: tablet_date, unit: 'tablet')
        ]
      }
    end

    def taper_step_config(date:, unit:)
      {
        'start_date' => date.iso8601,
        'end_date' => date.iso8601,
        'amount' => '1',
        'unit' => unit,
        'max_daily_doses' => 1,
        'min_hours_between_doses' => 24
      }
    end

    def build_source_tablet_taper_schedule(attributes = {})
      create(
        :schedule,
        {
          person: people(:john),
          medication: inventory_medication,
          dosage: tablet_option,
          dose_amount: 1,
          dose_unit: 'tablet',
          max_daily_doses: 1,
          min_hours_between_doses: 24,
          schedule_type: :tapering
        }.merge(attributes)
      )
    end

    def take_schedule_at(schedule:, travel_date:, taken_at:)
      result = nil

      travel_to travel_date.noon do
        expect do
          result = service.call(
            source: schedule,
            amount_override: nil,
            taken_from_medication_id: nil,
            user: user,
            taken_at: taken_at
          )
          expect(result.success).to be(true)
        end.to change(MedicationTake, :count).by(1)
      end

      result
    end

    def expect_inventory_supply(tablet:, capsule:, medication:)
      expect(
        [tablet_option.reload.current_supply, capsule_option.reload.current_supply,
         inventory_medication.reload.current_supply]
      ).to eq([tablet, capsule, medication])
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

    it 'matches schedule inventory using the effective dose option' do
      schedule = build_effective_capsule_schedule(inventory_medication)
      result = nil

      expect do
        result = call_service(source: schedule)
        expect(result.success).to be(true)
      end.to change(MedicationTake, :count).by(1)

      expect(tablet_option.reload.current_supply).to eq(56)
      expect(capsule_option.reload.current_supply).to eq(27)
      expect(inventory_medication.reload.current_supply).to eq(83)
    end

    it 'prefers the active taper step dose over the source dose option for inventory matching' do
      schedule = build_source_tablet_taper_schedule(schedule_config: effective_capsule_schedule_config)
      result = nil

      expect do
        result = call_service(source: schedule)
        expect(result.success).to be(true)
      end.to change(MedicationTake, :count).by(1)

      expect_inventory_supply(tablet: 56, capsule: 27, medication: 83)
    end

    it 'matches tapering inventory using the provided taken_at date' do
      capsule_date = Date.new(2026, 4, 21)
      tablet_date = Date.new(2026, 4, 24)
      schedule = build_source_tablet_taper_schedule(
        start_date: capsule_date,
        end_date: tablet_date,
        schedule_config: two_step_taper_schedule_config(capsule_date: capsule_date, tablet_date: tablet_date)
      )
      result = take_schedule_at(schedule: schedule, travel_date: tablet_date, taken_at: capsule_date.noon)

      expect(result.take.dose_amount).to eq(BigDecimal('1'))
      expect_inventory_supply(tablet: 56, capsule: 27, medication: 83)
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

  describe 'medication take events' do
    def expect_metric_payload(payloads, source:, error: nil)
      expect(payloads).to contain_exactly(
        include(
          environment: Rails.env.to_s,
          role: user.person.account.first_active_household_membership.role,
          route: nil,
          medicine_context_class: source.class.name,
          source_type: source.class.model_name.singular,
          error: error
        )
      )
    end

    def expect_dose_taken_payload(payloads, result:, source:, dose_amount:, source_type:)
      expect(payloads).to contain_exactly(
        include(
          take_id: result.take.id,
          source_type: source_type,
          source_id: source.id,
          person_id: source.person_id,
          medication_id: source.medication_id,
          inventory_medication_id: source.medication_id,
          inventory_location_id: source.medication.location_id,
          dose_amount: dose_amount,
          dose_unit: source.dose_unit,
          taken_at: result.take.taken_at
        )
      )
    end

    it 'publishes attempted and recorded metric events for a successful dose' do
      schedule = schedules(:john_paracetamol)
      result = nil

      travel_to Time.current.end_of_day - 1.minute do
        attempted_payloads = captured_event_payloads('take_attempted.med_tracker') do
          recorded_payloads = captured_event_payloads('take_recorded.med_tracker') do
            result = call_service(source: schedule)
          end

          expect_metric_payload(recorded_payloads, source: schedule)
        end

        expect(result.success).to be true
        expect_metric_payload(attempted_payloads, source: schedule)
      end
    end

    it 'publishes attempted and blocked metric events when rules prevent a dose' do
      schedule = schedules(:john_paracetamol)
      schedule.medication.update!(current_supply: 0)
      result = nil

      attempted_payloads = captured_event_payloads('take_attempted.med_tracker') do
        blocked_payloads = captured_event_payloads('take_blocked_by_rules.med_tracker') do
          result = call_service(source: schedule)
        end

        expect_metric_payload(blocked_payloads, source: schedule, error: 'out_of_stock')
      end

      expect(result.error).to eq(:out_of_stock)
      expect_metric_payload(attempted_payloads, source: schedule)
    end

    it 'publishes an error metric when take persistence fails' do
      schedule = schedules(:john_paracetamol)
      allow(schedule).to receive(:effective_dose_unit).and_return(nil)
      result = nil

      travel_to Time.current.end_of_day - 1.minute do
        payloads = captured_event_payloads('take_errors.med_tracker') do
          result = call_service(source: schedule)
        end

        expect(result.error).to eq(:create_failed)
        expect_metric_payload(payloads, source: schedule, error: 'create_failed')
      end
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
          dose_amount: 750.0,
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
        dose_amount: 1000.0,
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
          dose_amount: schedule.default_dose_amount,
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
        location: Location.create!(name: 'Selection Required Home', household: fixture_household)
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
      household: medication.household,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      current_supply: 12,
      reorder_threshold: 2
    )
  end

  def fixture_household
    user.person.account.first_active_household_membership.household
  end
end
