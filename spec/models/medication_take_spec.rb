# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTake do
  subject(:medication_take) { described_class.new(schedule: schedule, taken_at: Time.current, amount_ml: 10.0) }

  let(:medication) do
    Medication.create!(
      name: 'Lisinopril',
      location: Location.find_or_create_by!(name: 'Test Home'),
      dosage_amount: 10,
      dosage_unit: 'mg',
      current_supply: 50,
      reorder_threshold: 10
    )
  end

  let(:schedule) do
    create_schedule_for(person: person, medication: medication, dosage: default_dosage_for(medication))
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:taken_at) }
    it { is_expected.to validate_presence_of(:amount_ml) }
    it { is_expected.to validate_numericality_of(:amount_ml).is_greater_than(0) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:schedule).optional }
    it { is_expected.to belong_to(:person_medication).optional }
    it { is_expected.to belong_to(:taken_from_medication).class_name('Medication').optional }
    it { is_expected.to belong_to(:taken_from_location).class_name('Location').optional }
  end

  describe '#source_type' do
    it 'returns schedule for scheduled doses' do
      expect(described_class.new(schedule: schedule).source_type).to eq('schedule')
    end

    it 'returns person_medication for as-needed doses' do
      expect(described_class.new(person_medication: person_medication).source_type).to eq('person_medication')
    end
  end

  describe '#source_record_id' do
    it 'returns the schedule id for scheduled doses' do
      expect(described_class.new(schedule: schedule).source_record_id).to eq(schedule.id)
    end

    it 'returns the person_medication id for as-needed doses' do
      expect(described_class.new(person_medication: person_medication).source_record_id).to eq(person_medication.id)
    end
  end

  describe 'source validation' do
    context 'when neither schedule nor person_medication is set' do
      subject(:medication_take) { described_class.new(taken_at: Time.current, amount_ml: 10.0) }

      it 'is invalid' do
        expect(medication_take).not_to be_valid
        expect(medication_take.errors[:base]).to include(
          'Must have exactly one source (schedule or person_medication)'
        )
      end
    end

    context 'when both schedule and person_medication are set' do
      subject(:medication_take) do
        described_class.new(
          schedule: schedule,
          person_medication: person_medication,
          taken_at: Time.current,
          amount_ml: 10.0
        )
      end

      it 'is invalid' do
        expect(medication_take).not_to be_valid
        expect(medication_take.errors[:base]).to include(
          'Must have exactly one source (schedule or person_medication)'
        )
      end
    end

    context 'when only schedule is set' do
      subject(:medication_take) do
        described_class.new(
          schedule: schedule,
          taken_at: Time.current,
          amount_ml: 10.0
        )
      end

      it 'is valid' do
        expect(medication_take).to be_valid
      end
    end

    context 'when only person_medication is set' do
      subject(:medication_take) do
        described_class.new(
          person_medication: person_medication,
          taken_at: Time.current,
          amount_ml: 10.0
        )
      end

      let(:person_medication) do
        PersonMedication.create!(
          person: person,
          medication: medication,
          dose_amount: 10,
          dose_unit: 'mg'
        )
      end

      it 'is valid' do
        expect(medication_take).to be_valid
      end
    end
  end

  describe 'supply tracking' do
    before do
      medication.update!(current_supply: 100)
    end

    context 'when taking a dose from a schedule' do
      it 'deducts 1 from the medication current_supply' do
        expect do
          described_class.create!(
            schedule: schedule,
            taken_at: Time.current,
            amount_ml: 10.0
          )
        end.to change { medication.reload.current_supply }.from(100).to(99)
      end
    end

    context 'when taking a dose from a person_medication' do
      it 'deducts 1 from the medication current_supply' do
        expect do
          described_class.create!(
            person_medication: person_medication,
            taken_at: Time.current,
            amount_ml: 10.0
          )
        end.to change { medication.reload.current_supply }.from(100).to(99)
      end
    end

    context 'when taking a dose from an alternate location medication' do
      it 'deducts stock from the selected medication and records its location' do
        alternate_location = Location.create!(name: 'Grandma Alternate')
        alternate_medication = create_matching_medication(
          medication: medication,
          location: alternate_location
        )

        create_taken_from_schedule(
          schedule: schedule,
          taken_from_medication: alternate_medication,
          taken_from_location: alternate_location
        )

        expect(medication.reload.current_supply).to eq(100)
        expect(alternate_medication.reload.current_supply).to eq(11)
      end

      it 'stores the selected inventory source on the take' do
        alternate_location = Location.create!(name: 'Grandma Alternate')
        alternate_medication = create_matching_medication(
          medication: medication,
          location: alternate_location
        )
        take = create_taken_from_schedule(
          schedule: schedule,
          taken_from_medication: alternate_medication,
          taken_from_location: alternate_location
        )

        expect(take.inventory_medication).to eq(alternate_medication)
        expect(take.inventory_location).to eq(alternate_location)
      end
    end
  end

  describe 'low_stock_threshold_reached.med_tracker' do
    def captured_event_payloads(event_name, &)
      payloads = []
      subscriber = lambda do |*args|
        payloads << ActiveSupport::Notifications::Event.new(*args).payload
      end

      ActiveSupport::Notifications.subscribed(subscriber, event_name, &)

      payloads
    end

    def capture_low_stock_payloads(&)
      captured_event_payloads('low_stock_threshold_reached.med_tracker', &)
    end

    def expect_low_stock_payload(payloads, take:, expected:)
      expect(payloads).to contain_exactly(
        include(
          take_id: take.id,
          source_type: 'schedule',
          source_id: schedule.id,
          taken_at: take.taken_at,
          **expected
        )
      )
    end

    def capture_threshold_crossing_for(schedule:, medication:, location:)
      take = nil
      payloads = capture_low_stock_payloads do
        take = create_taken_from_schedule(
          schedule: schedule,
          taken_from_medication: medication,
          taken_from_location: location
        )
      end

      [payloads, take]
    end

    def expect_threshold_crossing_for(schedule:, medication:, location:, expected:)
      payloads, take = capture_threshold_crossing_for(
        schedule: schedule,
        medication: medication,
        location: location
      )

      expect_low_stock_payload(
        payloads,
        take: take,
        expected: expected
      )
    end

    def create_alternate_inventory_medication(current_supply: 3, reorder_threshold: 2)
      location = Location.create!(name: 'Event Alt')
      medication = create_matching_medication(
        medication: self.medication,
        location: location,
        current_supply: current_supply,
        reorder_threshold: reorder_threshold
      )

      [location, medication]
    end

    def expected_low_stock_event(medication:, location:, previous_current_supply:, current_supply:, reorder_threshold:)
      {
        medication_id: medication.id,
        location_id: location.id,
        previous_current_supply: previous_current_supply,
        current_supply: current_supply,
        reorder_threshold: reorder_threshold
      }
    end

    it 'publishes when stock crosses the reorder threshold' do
      medication.update!(current_supply: 11, reorder_threshold: 10)
      expect_threshold_crossing_for(
        schedule: schedule,
        medication: medication,
        location: medication.location,
        expected: expected_low_stock_event(
          medication: medication,
          location: medication.location,
          previous_current_supply: 11,
          current_supply: 10,
          reorder_threshold: 10
        )
      )
    end

    it 'does not publish when stock remains above the threshold' do
      medication.update!(current_supply: 12, reorder_threshold: 10)

      payloads = capture_low_stock_payloads do
        described_class.create!(
          schedule: schedule,
          taken_at: Time.current,
          amount_ml: 10.0
        )
      end

      expect(payloads).to be_empty
    end

    it 'does not publish when stock was already low' do
      medication.update!(current_supply: 10, reorder_threshold: 10)

      payloads = capture_low_stock_payloads do
        described_class.create!(
          schedule: schedule,
          taken_at: Time.current,
          amount_ml: 10.0
        )
      end

      expect(payloads).to be_empty
    end

    it 'does not publish when stock is untracked' do
      medication.update!(current_supply: nil, reorder_threshold: 10)

      payloads = capture_low_stock_payloads do
        described_class.create!(
          schedule: schedule,
          taken_at: Time.current,
          amount_ml: 10.0
        )
      end

      expect(payloads).to be_empty
    end

    it 'publishes for an alternate inventory medication when that stock crosses the threshold' do
      alternate_location, alternate_medication = create_alternate_inventory_medication

      expect_threshold_crossing_for(
        schedule: schedule,
        medication: alternate_medication,
        location: alternate_location,
        expected: expected_low_stock_event(
          medication: alternate_medication,
          location: alternate_location,
          previous_current_supply: 3,
          current_supply: 2,
          reorder_threshold: 2
        )
      )
    end
  end

  describe 'taken_from validation' do
    context 'when the selected medication uses a different identity' do
      it 'requires taken_from_medication to match the assigned medication identity' do
        alternate_location = Location.create!(name: 'Validation Alt')
        alternate_medication = create_matching_medication(medication: medication, location: alternate_location,
                                                          name: 'Different', current_supply: 10, reorder_threshold: 1)
        take = build_taken_from_schedule(
          schedule: schedule,
          taken_from_medication: alternate_medication,
          taken_from_location: alternate_location
        )

        expect(take).not_to be_valid
        expect(take.errors[:taken_from_medication]).to include('must match the assigned medication')
      end
    end
  end

  describe 'versioning' do
    fixtures :accounts, :people, :users

    let(:admin) { users(:admin) }
    let(:schedule) do
      person = people(:john)
      medication = Medication.create!(
        name: 'Test Medication',
        location: Location.find_or_create_by!(name: 'Versioning Home'),
        current_supply: 100,
        reorder_threshold: 10
      )
      dosage = Dosage.create!(
        medication: medication,
        amount: 10,
        unit: 'mg',
        frequency: 'daily',
        default_max_daily_doses: 1,
        default_min_hours_between_doses: 24,
        default_dose_cycle: :daily
      )

      Schedule.create!(
        person: person,
        medication: medication,
        dosage: dosage,
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
    end

    before do
      PaperTrail.request.whodunnit = admin.id
    end

    after do
      PaperTrail.request.whodunnit = nil
    end

    it 'creates version when medication is taken' do
      expect do
        described_class.create!(
          schedule: schedule,
          taken_at: Time.current,
          amount_ml: 5.0
        )
      end.to change(PaperTrail::Version.where(item_type: 'MedicationTake'), :count).by(1)

      version = PaperTrail::Version.where(item_type: 'MedicationTake').last
      expect(version.event).to eq('create')
      expect(version.item_type).to eq('MedicationTake')
    end

    it 'creates version on medication take update' do
      take = described_class.create!(
        schedule: schedule,
        taken_at: Time.current,
        amount_ml: 5.0
      )

      expect do
        take.update!(amount_ml: 10.0)
      end.to change(PaperTrail::Version, :count).by(1)

      version = take.versions.last
      expect(version.event).to eq('update')
      expect(version.object).to be_present
    end

    it 'tracks time changes for medication takes' do
      original_time = 2.hours.ago
      take = described_class.create!(
        schedule: schedule,
        taken_at: original_time,
        amount_ml: 5.0
      )

      new_time = 1.hour.ago
      take.update!(taken_at: new_time)

      version = take.versions.last
      reified = version.reify
      expect(reified.taken_at.to_i).to eq(original_time.to_i)
    end

    it 'associates version with current user' do
      take = described_class.create!(
        schedule: schedule,
        taken_at: Time.current,
        amount_ml: 5.0
      )
      expect(take.versions.last.whodunnit).to eq(admin.id.to_s)
    end

    it 'records IP address when controller_info is set' do
      PaperTrail.request.controller_info = { ip: '192.168.1.100' }

      take = described_class.create!(
        schedule: schedule,
        taken_at: Time.current,
        amount_ml: 5.0
      )

      expect(take.versions.last.ip).to eq('192.168.1.100')
    ensure
      PaperTrail.request.controller_info = nil
    end
  end # rubocop:enable RSpec/MultipleMemoizedHelpers

  def person
    @person ||= Person.create!(name: 'Jane Doe', date_of_birth: '1990-01-01')
  end

  def person_medication
    @person_medication ||= PersonMedication.create!(
      person: person,
      medication: medication,
      notes: 'Test notes'
    )
  end

  def default_dosage_for(medication)
    Dosage.create!(
      medication: medication,
      amount: 10,
      unit: 'mg',
      frequency: 'daily',
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    )
  end

  def create_schedule_for(person:, medication:, dosage:)
    Schedule.create!(
      person: person,
      medication: medication,
      dosage: dosage,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
  end

  def create_ambiguous_schedule(medication:)
    options = create_ambiguous_tracked_options(
      medication: medication,
      current_supply: 10,
      reorder_threshold: 2
    )

    create_schedule_for(person: person, medication: medication, dosage: options[:evening])
  end

  def create_ambiguous_tracked_options(medication:, current_supply:, reorder_threshold:)
    create_option_pair(
      medication: medication,
      current_supply_by_frequency: { 'Morning' => current_supply, 'Evening' => current_supply },
      reorder_threshold: reorder_threshold
    )
  end

  def create_alternate_medication_with_ambiguous_options(assigned_medication:, location_name:)
    alternate_medication = create_matching_medication(
      medication: assigned_medication,
      location: Location.create!(name: location_name),
      current_supply: 10,
      reorder_threshold: 2
    )

    options = create_option_pair(
      medication: alternate_medication,
      current_supply_by_frequency: { 'Morning' => 6, 'Evening' => 4 },
      reorder_threshold: 1
    )

    [alternate_medication, options]
  end

  def create_incomplete_alternate_medication(assigned_medication:)
    alternate_medication = create_matching_medication(
      medication: assigned_medication,
      location: Location.create!(name: 'Validation Dose Alt'),
      current_supply: 10,
      reorder_threshold: 2
    )
    create_option_pair(
      medication: alternate_medication,
      current_supply_by_frequency: { 'Morning' => 6 },
      reorder_threshold: 1
    )
    alternate_medication
  end

  def create_matching_medication(medication:, location:, **overrides)
    Medication.create!(
      {
        name: medication.name,
        location: location,
        dosage_amount: medication.dosage_amount,
        dosage_unit: medication.dosage_unit,
        current_supply: 12,
        reorder_threshold: 2
      }.merge(overrides)
    )
  end

  def create_option_pair(medication:, current_supply_by_frequency:, reorder_threshold:)
    current_supply_by_frequency.each_with_object({}) do |(frequency, current_supply), options|
      options[frequency.downcase.to_sym] = create_tracked_dosage(
        medication: medication,
        frequency: frequency,
        description: "#{frequency} tablet",
        current_supply: current_supply,
        reorder_threshold: reorder_threshold
      )
    end
  end

  def create_tracked_dosage(medication:, frequency:, description:, current_supply:, reorder_threshold:)
    create(
      :dosage,
      medication: medication,
      amount: 1,
      unit: 'tablet',
      frequency: frequency,
      description: description,
      current_supply: current_supply,
      reorder_threshold: reorder_threshold
    )
  end

  def create_taken_from_schedule(schedule:, taken_from_medication:, taken_from_location:)
    described_class.create!(
      schedule: schedule,
      taken_at: Time.current,
      amount_ml: 10.0,
      taken_from_medication: taken_from_medication,
      taken_from_location: taken_from_location
    )
  end

  def build_taken_from_schedule(schedule:, taken_from_medication:, taken_from_location:)
    described_class.new(
      schedule: schedule,
      taken_at: Time.current,
      amount_ml: 10.0,
      taken_from_medication: taken_from_medication,
      taken_from_location: taken_from_location
    )
  end
end
