# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FamilyDashboard::ScheduleQuery do
  fixtures :accounts, :people, :carer_relationships, :schedules, :person_medications, :medication_takes,
           :locations, :medications, :dosages

  let(:jane) { people(:jane) }
  let(:child) { people(:child_patient) }
  let(:query) { described_class.new([jane, child]) }

  describe '#call' do
    def count_source_queries(&)
      counts = Hash.new(0)

      subscriber = lambda do |_name, _start, _finish, _id, payload|
        sql = payload[:sql]
        next if payload[:cached] || payload[:name] == 'SCHEMA'

        counts[:schedules] += 1 if sql.include?('FROM "schedules"') && sql.include?('"schedules"."person_id"')
        counts[:person_medications] += 1 if sql.include?('FROM "person_medications"') &&
                                            sql.include?('"person_medications"."person_id"')
      end

      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
      counts
    end

    it 'bulk loads dose sources for the dashboard family' do
      counts = count_source_queries { query.call }

      expect(counts[:schedules]).to eq(1)
      expect(counts[:person_medications]).to eq(1)
    end

    it 'returns an aggregated list of doses for the person and their dependents' do
      results = query.call
      expect(results).to be_an(Array)

      # Jane and her child should be in the results
      people_in_results = results.pluck(:person).uniq
      expect(people_in_results).to contain_exactly(jane, child)
    end

    it 'includes schedule doses from both the parent and the child' do
      results = query.call
      schedules = results.pluck(:source).grep(Schedule)

      expect(schedules).to include(schedules(:jane_ibuprofen))
      expect(schedules).to include(schedules(:patient_schedule)) # child_patient's ibuprofen
    end

    it 'includes person_medication (non-schedule) doses' do
      results = query.call
      pms = results.pluck(:source).grep(PersonMedication)
      expect(pms).to include(person_medications(:jane_vitamin_d))
    end

    it 'keeps routine direct medications due after a previous-day dose' do
      travel_to Time.zone.parse('2026-05-05 08:00:00') do
        routine_medication = create_routine_person_medication
        MedicationTake.create!(
          person_medication: routine_medication,
          taken_at: Time.zone.parse('2026-05-04 20:00:00'),
          dose_amount: 500,
          dose_unit: 'mg'
        )

        rows = described_class.new([people(:john)]).call.select { |row| row[:source] == routine_medication }

        expect(rows.pluck(:status)).to include(:upcoming)
      end
    end

    it 'does not emit a reset-time row after today routine direct medication has been taken' do
      travel_to Time.zone.parse('2026-05-05 20:00:00') do
        routine_medication = create_routine_person_medication
        MedicationTake.create!(
          person_medication: routine_medication,
          taken_at: Time.zone.parse('2026-05-05 08:00:00'),
          dose_amount: 500,
          dose_unit: 'mg'
        )

        rows = described_class.new([people(:john)]).call.select { |row| row[:source] == routine_medication }

        expect(rows.pluck(:status)).to eq([:taken])
      end
    end

    it 'keeps a once-daily scheduled medication due after a late previous-day dose' do
      travel_to Time.zone.parse('2026-05-12 10:20:00') do
        schedule = create_daily_scheduled_medication
        MedicationTake.create!(
          schedule: schedule,
          taken_at: Time.zone.parse('2026-05-11 20:00:00'),
          dose_amount: 1,
          dose_unit: 'tablet'
        )

        rows = described_class.new([people(:john)]).call.select { |row| row[:source] == schedule }

        expect(rows.pluck(:status)).to eq([:upcoming])
      end
    end

    it 'groups PRN schedules under as-needed medications instead of routine tasks' do
      prn_schedule = create_prn_schedule
      query = described_class.new([people(:john)])

      routine_rows = query.call.select { |row| row[:source] == prn_schedule }
      as_needed_rows = query.as_needed_by_person.fetch(people(:john)).select { |row| row[:source] == prn_schedule }

      expect(routine_rows).to be_empty
      expect(as_needed_rows.pluck(:status)).to eq([:available])
    end

    it 'marks PRN availability as max reached when the daily limit is exhausted' do
      prn_schedule = create_prn_schedule(max_daily_doses: 1)
      MedicationTake.create!(schedule: prn_schedule, taken_at: Time.current, dose_amount: 1000, dose_unit: 'mg')
      query = described_class.new([people(:john)])

      query.call
      rows = query.as_needed_by_person.fetch(people(:john)).select { |row| row[:source] == prn_schedule }

      expect(rows.pluck(:status)).to eq([:max_reached])
    end

    it 'adds daily dose progress to routine and as-needed rows' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        routine_medication = create_routine_person_medication
        prn_schedule = create_prn_schedule(max_daily_doses: 4)
        MedicationTake.create!(person_medication: routine_medication, taken_at: 1.hour.ago, dose_amount: 500,
                               dose_unit: 'mg')
        MedicationTake.create!(schedule: prn_schedule, taken_at: 2.hours.ago, dose_amount: 1000, dose_unit: 'mg')

        query = described_class.new([people(:john)])
        routine_rows = query.call.select { |row| row[:source] == routine_medication }
        prn_rows = query.as_needed_by_person.fetch(people(:john)).select { |row| row[:source] == prn_schedule }

        expect(routine_rows.first).to include(daily_dose_count: 1, daily_dose_limit: 1)
        expect(prn_rows.first).to include(daily_dose_count: 1, daily_dose_limit: 4)
      end
    end

    it 'exposes today take history for each source row' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        prn_schedule = create_prn_schedule(max_daily_doses: 4)
        take = MedicationTake.create!(schedule: prn_schedule, taken_at: Time.zone.parse('2026-05-05 09:15:00'),
                                      dose_amount: 1000, dose_unit: 'mg')

        query = described_class.new([people(:john)])
        query.call
        row = query.as_needed_by_person.fetch(people(:john)).find { |candidate| candidate[:source] == prn_schedule }

        expect(row[:today_takes]).to contain_exactly(take)
      end
    end

    it 'preloads medication sources for today take history' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        routine_medication = create_routine_person_medication
        prn_schedule = create_prn_schedule(max_daily_doses: 4)
        MedicationTake.create!(person_medication: routine_medication, taken_at: 1.hour.ago, dose_amount: 500,
                               dose_unit: 'mg')
        MedicationTake.create!(schedule: prn_schedule, taken_at: 2.hours.ago, dose_amount: 1000, dose_unit: 'mg')

        query = described_class.new([people(:john)])
        query.call
        rows = query.routine_tasks_by_person.fetch(people(:john)) +
               query.as_needed_by_person.fetch(people(:john))
        takes = rows.flat_map { |row| row[:today_takes] }

        expect(takes).to all(satisfy { |take| medication_source_preloaded?(take) })
      end
    end

    it 'orders actionable routine rows before already taken rows' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        schedule = create_multiple_daily_scheduled_medication
        MedicationTake.create!(
          schedule: schedule,
          taken_at: Time.zone.parse('2026-05-05 08:00:00'),
          dose_amount: 1,
          dose_unit: 'tablet'
        )

        rows = described_class.new([people(:john)]).call.select { |row| row[:source] == schedule }

        expect(rows.pluck(:status)).to eq(%i[upcoming taken])
      end
    end

    it 'orders cooldown rows by next available time' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        sooner = create_interval_scheduled_medication
        later = create_interval_scheduled_medication
        MedicationTake.create!(schedule: later, taken_at: 1.hour.ago, dose_amount: 1, dose_unit: 'tablet')
        MedicationTake.create!(schedule: sooner, taken_at: 2.hours.ago, dose_amount: 1, dose_unit: 'tablet')

        rows = described_class.new([people(:john)]).call.select do |row|
          [sooner, later].include?(row[:source]) && row[:status] == :cooldown
        end

        expect(rows.pluck(:source)).to eq([sooner, later])
        expect(rows.pluck(:status)).to all(eq(:cooldown))
      end
    end

    it 'orders completed and exhausted rows after blocked rows' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        taken_schedule, exhausted_schedule, out_of_stock_schedule = create_completed_priority_sources

        query = described_class.new([people(:john)])
        query.call
        rows = dashboard_rows_for(query, [taken_schedule, exhausted_schedule, out_of_stock_schedule])

        expect(rows.pluck(:source)).to eq([out_of_stock_schedule, taken_schedule, exhausted_schedule])
        expect(rows.pluck(:status)).to eq(%i[out_of_stock taken max_reached])
      end
    end

    it 'uses deterministic person and source tie-breakers for same-priority rows' do
      travel_to Time.zone.parse('2026-05-05 12:00:00') do
        lower_person = people(:john)
        higher_person = people(:jane)
        higher_person_schedule = create_daily_scheduled_medication(person: higher_person)
        lower_person_schedule = create_daily_scheduled_medication(person: lower_person)

        rows = described_class.new([higher_person, lower_person]).call.select do |row|
          [higher_person_schedule, lower_person_schedule].include?(row[:source])
        end

        expect(rows.pluck(:source)).to eq([lower_person_schedule, higher_person_schedule])
      end
    end

    it 'correctly identifies doses already taken today' do
      results = query.call
      taken_doses = results.select { |r| r[:status] == :taken }

      # Jane has jane_evening_ibuprofen and jane_morning_ibuprofen in fixtures
      jane_takes = taken_doses.select { |r| r[:person] == jane }
      expect(jane_takes).not_to be_empty
      expect(jane_takes.first[:taken_at]).to be_present
    end

    it 'filters out inactive schedules' do
      # Create an inactive schedule for Jane
      Schedule.create!(
        person: jane,
        medication: medications(:paracetamol),
        dose_amount: 1000,
        dose_unit: 'mg',
        start_date: 1.year.ago,
        end_date: 1.month.ago,
        frequency: 'Daily'
      )

      results = query.call
      active_schedules = results.pluck(:source).grep(Schedule)
      active_schedules.each do |p|
        expect(p.start_date).to be <= Time.zone.today
        expect(p.end_date).to be >= Time.zone.today
      end
    end

    context 'when a schedule has no timing restrictions' do
      let!(:schedule) do
        Schedule.create!(
          person: people(:john),
          medication: medications(:paracetamol),
          dose_amount: 1000,
          dose_unit: 'mg',
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'As needed',
          max_daily_doses: nil,
          min_hours_between_doses: nil
        )
      end

      it 'does not emit an upcoming row for a source with no timing restrictions' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == schedule && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end

    context 'when a schedule is on cooldown' do
      let!(:schedule) do
        Schedule.create!(
          person: people(:john),
          medication: medications(:ibuprofen),
          dose_amount: 400,
          dose_unit: 'mg',
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'Every 2 hours',
          min_hours_between_doses: 2,
          max_daily_doses: nil
        )
      end

      before do
        MedicationTake.create!(schedule: schedule, taken_at: 1.hour.ago, dose_amount: 400)
      end

      it 'emits a row with :cooldown status' do
        results = described_class.new([people(:john)]).call
        cooldown_rows = results.select { |r| r[:source] == schedule && r[:status] == :cooldown }
        expect(cooldown_rows).not_to be_empty
      end

      it 'does not emit an :upcoming row for the same source' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == schedule && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end

    context 'when max daily doses are reached' do
      let!(:schedule) do
        Schedule.create!(
          person: people(:john),
          medication: medications(:paracetamol),
          dose_amount: 1000,
          dose_unit: 'mg',
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'As needed',
          max_daily_doses: 2
        )
      end

      before do
        2.times { MedicationTake.create!(schedule: schedule, taken_at: Time.current, dose_amount: 500) }
      end

      it 'does not emit an :upcoming row when daily max is reached' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == schedule && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end

    context 'when medication is out of stock' do
      let!(:schedule) do
        oos_medication = Medication.create!(name: 'OOS Med', current_supply: 0, reorder_threshold: 2,
                                            location: locations(:home))
        Dosage.create!(
          medication: oos_medication,
          amount: 10,
          unit: 'mg',
          frequency: 'daily',
          default_max_daily_doses: 1,
          default_min_hours_between_doses: 24,
          default_dose_cycle: :daily
        )
        Schedule.create!(
          person: people(:john),
          medication: oos_medication,
          dose_amount: 10,
          dose_unit: 'mg',
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'daily',
          min_hours_between_doses: 4
        )
      end

      it 'emits a row with :out_of_stock status' do
        results = described_class.new([people(:john)]).call
        oos_rows = results.select { |r| r[:source] == schedule && r[:status] == :out_of_stock }
        expect(oos_rows).not_to be_empty
      end

      it 'does not emit an :upcoming row when out of stock' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == schedule && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end
  end

  def create_routine_person_medication
    PersonMedication.create!(
      person: people(:john),
      medication: medications(:vitamin_c),
      dose_amount: 500,
      dose_unit: 'mg',
      max_daily_doses: 1,
      min_hours_between_doses: nil,
      administration_kind: :routine
    )
  end

  def create_prn_schedule(max_daily_doses: 4, min_hours_between_doses: 4)
    Schedule.create!(
      person: people(:john),
      medication: medications(:paracetamol),
      dose_amount: 1000,
      dose_unit: 'mg',
      start_date: Time.zone.today,
      end_date: 1.year.from_now.to_date,
      frequency: 'As needed',
      schedule_type: :prn,
      max_daily_doses: max_daily_doses,
      min_hours_between_doses: min_hours_between_doses
    )
  end

  def create_daily_scheduled_medication(person: people(:john))
    medication = medications(:vitamin_c).tap { |candidate| candidate.update!(current_supply: 30) }

    create(
      :schedule,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: 1,
      dose_unit: 'tablet',
      frequency: 'Once daily',
      schedule_type: :daily,
      schedule_config: { 'times' => ['08:00'] },
      max_daily_doses: 1,
      min_hours_between_doses: 24,
      dose_cycle: :daily
    )
  end

  def create_multiple_daily_scheduled_medication
    create(
      :schedule,
      person: people(:john),
      medication: create(:medication, current_supply: 30),
      dosage: nil,
      dose_amount: 1,
      dose_unit: 'tablet',
      frequency: 'Twice daily',
      schedule_type: :multiple_daily,
      schedule_config: { 'times' => %w[08:00 18:00] },
      max_daily_doses: 2,
      min_hours_between_doses: nil,
      dose_cycle: :daily
    )
  end

  def create_interval_scheduled_medication
    create(
      :schedule,
      person: people(:john),
      medication: create(:medication, current_supply: 30),
      dosage: nil,
      dose_amount: 1,
      dose_unit: 'tablet',
      frequency: 'Every 4 hours',
      schedule_type: :daily,
      schedule_config: {},
      max_daily_doses: nil,
      min_hours_between_doses: 4,
      dose_cycle: :daily
    )
  end

  def create_out_of_stock_scheduled_medication
    create(
      :schedule,
      person: people(:john),
      medication: create(:medication, current_supply: 0, reorder_threshold: 2),
      dosage: nil,
      dose_amount: 1,
      dose_unit: 'tablet',
      frequency: 'Daily',
      schedule_type: :daily,
      schedule_config: {},
      max_daily_doses: 1,
      min_hours_between_doses: nil,
      dose_cycle: :daily
    )
  end

  def create_completed_priority_sources
    taken_schedule = create_daily_scheduled_medication
    exhausted_schedule = create_prn_schedule(max_daily_doses: 1)
    out_of_stock_schedule = create_out_of_stock_scheduled_medication

    record_schedule_take(taken_schedule, taken_at: Time.zone.parse('2026-05-05 08:00:00'), amount: 1, unit: 'tablet')
    record_schedule_take(exhausted_schedule, taken_at: Time.current, amount: 1000, unit: 'mg')

    [taken_schedule, exhausted_schedule, out_of_stock_schedule]
  end

  def record_schedule_take(schedule, taken_at:, amount:, unit:)
    MedicationTake.create!(schedule: schedule, taken_at: taken_at, dose_amount: amount, dose_unit: unit)
  end

  def dashboard_rows_for(query, sources)
    (
      query.routine_tasks_by_person.fetch(people(:john)) +
      query.as_needed_by_person.fetch(people(:john))
    ).select { |row| sources.include?(row[:source]) }
  end

  def medication_source_preloaded?(take)
    if take.schedule_id.present?
      take.association(:schedule).loaded? && take.schedule.association(:medication).loaded?
    else
      take.association(:person_medication).loaded? && take.person_medication.association(:medication).loaded?
    end
  end
end
