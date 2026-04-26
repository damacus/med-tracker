# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Schedule do
  fixtures :accounts, :schedules, :people, :locations, :medications, :dosages

  describe '#dose_constraints' do
    let(:active_date) { Date.new(2026, 4, 24) }
    let(:schedule) do
      create(:schedule, max_daily_doses: 3, min_hours_between_doses: 4)
    end
    let(:tapering_schedule) do
      create(
        :schedule,
        dosage: nil,
        source_dosage_option: nil,
        schedule_type: :tapering,
        dose_amount: 10,
        dose_unit: 'mg',
        max_daily_doses: 4,
        min_hours_between_doses: 4,
        start_date: active_date,
        end_date: active_date,
        schedule_config: tapering_schedule_config
      )
    end
    let(:tapering_schedule_config) do
      {
        'taper_steps' => [
          {
            'start_date' => active_date.iso8601,
            'end_date' => active_date.iso8601,
            'amount' => '5',
            'unit' => 'mg',
            'max_daily_doses' => 1,
            'min_hours_between_doses' => 24
          }
        ]
      }
    end

    it 'returns a value object built from the persisted timing fields' do
      expect(schedule.dose_constraints).to have_attributes(
        max_daily_doses: 3,
        min_hours_between_doses: 4
      )
    end

    it 'uses the active taper step timing values' do
      travel_to active_date.noon do
        expect(tapering_schedule.dose_constraints).to have_attributes(
          max_daily_doses: 1,
          min_hours_between_doses: 24
        )
      end
    end
  end

  describe '#timing_restrictions?' do
    it 'delegates to dose_constraints' do
      schedule = create(:schedule, max_daily_doses: 1)

      expect(schedule.timing_restrictions?).to be true
    end
  end

  describe 'active flag' do
    let(:schedule) { schedules(:john_paracetamol) }

    it 'is active by default' do
      new_schedule = described_class.new(
        person: people(:john),
        medication: medications(:paracetamol),
        dose_amount: 1000,
        dose_unit: 'mg',
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
      new_schedule.save
      expect(new_schedule.active).to be true
    end

    it 'can be set to inactive' do
      schedule.update(active: false)
      expect(schedule.active).to be false
    end

    it 'can be reactivated' do
      schedule.update(active: false)
      schedule.update(active: true)
      expect(schedule.active).to be true
    end
  end

  describe 'validations' do
    subject(:schedule) do
      described_class.new(
        person: person,
        medication: medication,
        dose_amount: dosage.amount,
        dose_unit: dosage.unit,
        start_date: Time.zone.today,
        end_date: Time.zone.today + 30.days
      )
    end

    let(:person) do
      Person.create!(
        name: 'Jane Doe',
        email: 'jane@example.com',
        date_of_birth: Date.new(1990, 1, 1)
      )
    end
    let(:location) { Location.create!(name: 'Schedule Test Home') }
    let(:medication) do
      Medication.create!(
        name: 'Lisinopril',
        location: location,
        current_supply: 50,
        reorder_threshold: 10
      )
    end
    let(:dosage) { create(:dosage, medication: medication, amount: 10, unit: 'mg', frequency: 'daily') }

    it { is_expected.to validate_presence_of(:start_date) }
    it { is_expected.to validate_presence_of(:end_date) }

    it 'is invalid if end_date is before start_date' do
      schedule.end_date = schedule.start_date - 1.day
      expect(schedule).not_to be_valid
    end
  end

  describe '#can_administer?' do
    let(:location) { Location.create!(name: 'Administer Test Home') }
    let(:medication) do
      Medication.create!(name: 'TestMed', location: location, current_supply: 10,
                         reorder_threshold: 2)
    end
    let(:person) do
      Person.create!(name: 'Test Person', email: 'test-administer@example.com', date_of_birth: Date.new(1990, 1, 1))
    end
    let(:dosage) do
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
    let(:schedule) do
      described_class.create!(
        person: person, medication: medication, dose_amount: dosage.amount, dose_unit: dosage.unit,
        start_date: Time.zone.today, end_date: Time.zone.today + 30.days
      )
    end

    context 'when can take now and medication in stock' do
      it 'returns true' do
        expect(schedule.can_administer?).to be true
      end
    end

    context 'when medication is out of stock' do
      before { medication.update!(current_supply: 0) }

      it 'returns false' do
        expect(schedule.can_administer?).to be false
      end
    end

    context 'when medication current_supply is nil (untracked)' do
      before { medication.update!(current_supply: nil) }

      it 'returns true' do
        expect(schedule.can_administer?).to be true
      end
    end
  end

  describe '#administration_blocked_reason' do
    let(:location) { Location.create!(name: 'Blocked Test Home') }
    let(:medication) do
      Medication.create!(name: 'TestMed2', location: location, current_supply: 0,
                         reorder_threshold: 2)
    end
    let(:person) do
      Person.create!(name: 'Test Person2', email: 'test-reason@example.com', date_of_birth: Date.new(1990, 1, 1))
    end
    let(:dosage) do
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
    let(:schedule) do
      described_class.create!(
        person: person, medication: medication, dose_amount: dosage.amount, dose_unit: dosage.unit,
        start_date: Time.zone.today, end_date: Time.zone.today + 30.days
      )
    end

    it 'returns :out_of_stock when medication has no supply' do
      expect(schedule.administration_blocked_reason).to eq(:out_of_stock)
    end
  end

  describe '#active?' do
    let(:schedule) do
      described_class.new(
        start_date: start_date,
        end_date: end_date
      )
    end
    let(:start_date) { Time.zone.today }
    let(:end_date) { Time.zone.today + 30.days }

    context 'when today is between start_date and end_date (inclusive)' do
      it 'returns true' do
        expect(schedule.active?).to be true
      end
    end

    context 'when today is before start_date' do
      let(:start_date) { Time.zone.today + 5.days }

      it 'returns false' do
        expect(schedule.active?).to be false
      end
    end

    context 'when today is after end_date' do
      let(:start_date) { Time.zone.today - 30.days }
      let(:end_date) { Time.zone.today - 5.days }

      it 'returns false' do
        expect(schedule.active?).to be false
      end
    end

    context 'when start_date is nil' do
      let(:start_date) { nil }

      it 'returns false' do
        expect(schedule.active?).to be false
      end
    end

    context 'when end_date is nil' do
      let(:end_date) { nil }

      it 'returns false' do
        expect(schedule.active?).to be false
      end
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person) }
    it { is_expected.to have_many(:medication_takes).dependent(:destroy) }

    it 'destroys medication takes when destroyed' do
      schedule = create(:schedule)
      take = create(:medication_take, :for_schedule, schedule: schedule)

      expect { schedule.destroy! }
        .to change { MedicationTake.exists?(take.id) }.from(true).to(false)
    end
  end

  describe '#default_dose_amount' do
    it 'returns the persisted schedule dose amount' do
      schedule = build(:schedule, dose_amount: 12.5, dose_unit: 'ml')

      expect(schedule.default_dose_amount).to eq(12.5)
    end
  end

  describe 'schedule configuration' do
    let(:monday) { Date.new(2026, 4, 20) }
    let(:taper_steps) do
      [
        {
          'start_date' => '2026-04-20',
          'end_date' => '2026-04-22',
          'amount' => '10',
          'unit' => 'mg',
          'max_daily_doses' => 3,
          'min_hours_between_doses' => 6
        },
        {
          'start_date' => '2026-04-23',
          'end_date' => '2026-04-26',
          'amount' => '5',
          'unit' => 'mg',
          'max_daily_doses' => 1,
          'min_hours_between_doses' => 24
        }
      ]
    end
    let(:tapering_schedule) do
      build(
        :schedule,
        schedule_type: :tapering,
        schedule_config: { 'taper_steps' => taper_steps },
        dose_amount: 20,
        dose_unit: 'mg',
        start_date: monday,
        end_date: monday + 1.week
      )
    end

    it 'defaults to daily schedule behavior for legacy-compatible rows' do
      schedule = build(:schedule, start_date: monday, end_date: monday + 1.week, max_daily_doses: 4)

      expect(schedule.schedule_type).to eq('daily')
      expect(schedule.schedule_config).to eq({})
      expect(schedule.applies_on?(monday + 1.day)).to be true
      expect(schedule.expected_doses_on(monday + 1.day)).to eq(4)
    end

    it 'supports multiple daily doses from configured times' do
      schedule = build(
        :schedule,
        schedule_type: :multiple_daily,
        schedule_config: { 'times' => %w[08:00 14:00 20:00] },
        start_date: monday,
        end_date: monday + 1.week
      )

      expect(schedule.applies_on?(monday)).to be true
      expect(schedule.expected_doses_on(monday)).to eq(3)
    end

    it 'supports weekly schedules using weekday names and integers' do
      schedule = build(
        :schedule,
        schedule_type: :weekly,
        schedule_config: { 'weekdays' => ['Monday', 3] },
        start_date: monday,
        end_date: monday + 1.week,
        max_daily_doses: 1
      )

      expect(schedule.applies_on?(monday)).to be true
      expect(schedule.applies_on?(monday + 2.days)).to be true
      expect(schedule.applies_on?(monday + 1.day)).to be false
      expect(schedule.expected_doses_on(monday + 2.days)).to eq(1)
    end

    it 'supports schedules on specific ISO dates' do
      schedule = build(
        :schedule,
        schedule_type: :specific_dates,
        schedule_config: { 'dates' => %w[2026-04-21 2026-04-24] },
        start_date: monday,
        end_date: monday + 1.week
      )

      expect(schedule.applies_on?(Date.new(2026, 4, 21))).to be true
      expect(schedule.applies_on?(Date.new(2026, 4, 22))).to be false
    end

    it 'treats PRN schedules as active but not expected' do
      schedule = build(:schedule, schedule_type: :prn, start_date: monday, end_date: monday + 1.week)

      expect(schedule.applies_on?(monday + 1.day)).to be true
      expect(schedule.expected_doses_on(monday + 1.day)).to eq(0)
    end

    it 'supports every-other-day schedules from the start date' do
      schedule = build(:schedule, schedule_type: :every_other_day, start_date: monday, end_date: monday + 1.week)

      expect(schedule.applies_on?(monday)).to be true
      expect(schedule.applies_on?(monday + 1.day)).to be false
      expect(schedule.applies_on?(monday + 2.days)).to be true
    end

    it 'uses the current taper step for effective dose and restrictions' do
      date = Date.new(2026, 4, 24)

      aggregate_failures do
        expect(tapering_schedule.applies_on?(date)).to be true
        expect(tapering_schedule.effective_dose_amount(date)).to eq(BigDecimal('5'))
        expect(tapering_schedule.effective_dose_unit(date)).to eq('mg')
        expect(tapering_schedule.effective_max_daily_doses(date)).to eq(1)
        expect(tapering_schedule.effective_min_hours_between_doses(date)).to eq(24)
        expect(tapering_schedule.expected_doses_on(date)).to eq(1)
      end
    end
  end

  describe '#frequency vs #dose_cycle' do
    let(:medication) { medications(:paracetamol) }
    let(:dosage) do
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
    let(:schedule) do
      described_class.create!(
        person: people(:john),
        medication: medication,
        dose_amount: dosage.amount,
        dose_unit: dosage.unit,
        frequency: 'Up to 3 times daily, at least 4h apart',
        dose_cycle: :daily,
        start_date: Time.zone.today,
        end_date: 1.year.from_now.to_date,
        max_daily_doses: 3,
        min_hours_between_doses: 4
      )
    end

    it 'frequency is a free-text display label, independent of dose_cycle' do
      expect(schedule.frequency).to eq('Up to 3 times daily, at least 4h apart')
      expect(schedule.dose_cycle).to eq('daily')
      expect(schedule.cycle_period).to be_a(ActiveSupport::Duration)
    end

    it 'frequency and dose_cycle can diverge independently' do
      schedule.update!(dose_cycle: :weekly, frequency: 'Once a week on Mondays')
      expect(schedule.dose_cycle).to eq('weekly')
      expect(schedule.frequency).to eq('Once a week on Mondays')
    end
  end
end
