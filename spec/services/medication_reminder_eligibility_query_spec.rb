# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReminderEligibilityQuery do
  let(:person) { create(:person) }
  let(:now) { Time.zone.local(2026, 6, 9, 10, 0, 0) }
  let(:today) { now.to_date }

  before { travel_to(now) }

  def build_query(scheduled_time: nil)
    described_class.new(person: person, scheduled_time: scheduled_time, now: now)
  end

  def schedule_with_times(times:, medication: nil, takes: [], frequency: 'Daily', **attrs)
    med = medication || create(:medication)
    schedule = create(:schedule, person: person, medication: med,
                                 schedule_type: :multiple_daily, frequency: frequency,
                                 schedule_config: { 'times' => times },
                                 start_date: today - 1.day, end_date: today + 30.days,
                                 **attrs)
    takes.each { |taken_at| create(:medication_take, :for_schedule, schedule: schedule, taken_at: taken_at) }
    schedule
  end

  def prn_schedule(medication: nil)
    med = medication || create(:medication)
    create(:schedule, person: person, medication: med,
                      schedule_type: :prn, frequency: 'As needed',
                      schedule_config: {},
                      start_date: today - 1.day, end_date: today + 30.days)
  end

  def daily_schedule(medication: nil, takes: [])
    med = medication || create(:medication)
    schedule = create(:schedule, person: person, medication: med,
                                 schedule_type: :daily, frequency: 'Daily',
                                 schedule_config: { 'times' => ['08:00'] },
                                 start_date: today - 1.day, end_date: today + 30.days)
    takes.each { |taken_at| create(:medication_take, :for_schedule, schedule: schedule, taken_at: taken_at) }
    schedule
  end

  describe '#medication_names' do
    it 'returns an empty array when there are no schedules' do
      expect(build_query.medication_names).to eq([])
    end

    it 'returns medication names for due schedules' do
      schedule = daily_schedule
      names = build_query.medication_names
      expect(names).to include(schedule.medication_name)
    end

    it 'does not include PRN schedule medications' do
      prn = prn_schedule
      names = build_query.medication_names
      expect(names).not_to include(prn.medication_name)
    end

    it 'does not include medications already taken in current cycle' do
      taken_schedule = daily_schedule(takes: [now - 1.hour])
      names = build_query.medication_names
      expect(names).not_to include(taken_schedule.medication_name)
    end

    it 'does not include medications from expired schedules' do
      med = create(:medication)
      _expired = create(:schedule, person: person, medication: med,
                                   schedule_type: :daily, frequency: 'Daily',
                                   schedule_config: { 'times' => ['08:00'] },
                                   start_date: today - 10.days, end_date: today - 1.day)
      expect(build_query.medication_names).not_to include(med.name)
    end

    it 'does not include medications from paused schedules' do
      schedule = daily_schedule
      schedule.pause!

      expect(build_query.medication_names).not_to include(schedule.medication_name)
    end

    it 'deduplicates medication names' do
      # Same medication on two active schedules
      med = create(:medication)
      daily_schedule(medication: med)
      schedule_with_times(times: ['12:00'], medication: med)

      names = build_query.medication_names
      expect(names.count(med.name)).to be <= 1
    end

    context 'with routine person_medications' do
      it 'includes medications from routine person_medications not yet taken today' do
        medication = create(:medication)
        create(:person_medication, :routine, person: person, medication: medication, max_daily_doses: 1)

        names = build_query.medication_names
        expect(names).to include(medication.display_name)
      end

      it 'excludes as_needed person_medications' do
        medication = create(:medication)
        pm = create(:person_medication, :as_needed, person: person, medication: medication)

        names = build_query.medication_names
        # as_needed person_medications are not included in due_person_medication_names
        expect(names).not_to include(pm.medication.display_name)
      end

      it 'excludes routine person_medications already taken today' do
        medication = create(:medication)
        pm = create(:person_medication, :routine, person: person, medication: medication, max_daily_doses: 1)
        create(:medication_take, :for_person_medication, person_medication: pm, taken_at: now - 1.hour)

        names = build_query.medication_names
        expect(names).not_to include(medication.display_name)
      end

      it 'excludes paused routine person_medications' do
        medication = create(:medication)
        create(:person_medication, :routine, person: person, medication: medication, active: false,
                                             max_daily_doses: 1)

        expect(build_query.medication_names).not_to include(medication.display_name)
      end
    end

    context 'with scheduled_time provided' do
      it 'includes medications due at the given scheduled time' do
        schedule = schedule_with_times(times: ['08:00'])

        names = build_query(scheduled_time: '08:00').medication_names
        expect(names).to include(schedule.medication_name)
      end

      it 'excludes medications not scheduled at the given time' do
        schedule = schedule_with_times(times: ['14:00'])

        names = build_query(scheduled_time: '08:00').medication_names
        expect(names).not_to include(schedule.medication_name)
      end

      it 'does not include routine person_medications when scheduled_time is set' do
        medication = create(:medication)
        create(:person_medication, :routine, person: person, medication: medication, max_daily_doses: 1)

        names = build_query(scheduled_time: '08:00').medication_names
        expect(names).not_to include(medication.display_name)
      end
    end
  end

  describe '#configured_times' do
    it 'returns an empty array when there are no due schedules' do
      expect(build_query.configured_times).to eq([])
    end

    it 'returns configured times for due schedules' do
      schedule_with_times(times: %w[08:00 12:00])

      times = build_query.configured_times
      expect(times).to include('08:00', '12:00')
    end

    it 'deduplicates times across multiple schedules' do
      schedule_with_times(times: ['08:00'])
      schedule_with_times(times: ['08:00'])

      times = build_query.configured_times
      expect(times.count('08:00')).to eq(1)
    end

    it 'excludes times from fully-taken schedules' do
      # A schedule whose dose has already been taken is not "due",
      # so none of its configured times appear in configured_times.
      schedule_with_times(times: %w[08:00], takes: [now - 2.hours])

      times = build_query.configured_times
      expect(times).not_to include('08:00')
    end

    it 'excludes times from paused schedules' do
      schedule = schedule_with_times(times: %w[08:00])
      schedule.pause!

      expect(build_query.configured_times).not_to include('08:00')
    end
  end
end
