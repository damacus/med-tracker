# frozen_string_literal: true

module FamilyDashboard
  # Query object to fetch a 24-hour medication schedule for a person and their dependents
  class ScheduleQuery
    Result = Data.define(:routine_tasks, :routine_tasks_by_person, :as_needed_by_person)

    delegate :routine_tasks, :routine_tasks_by_person, :as_needed_by_person, to: :result

    attr_reader :current_user

    def initialize(people, current_user: nil)
      @people = Array(people)
      @current_user = current_user
    end

    def call
      result.routine_tasks
    end

    private

    def result
      @result ||= build_result
    end

    def build_result
      # 1. Fetch all active schedules and person_medications for these people first
      # to get the IDs for take preloading
      @person_ids = @people.map(&:id)
      @all_schedules = fetch_active_schedules
      @all_person_medications = fetch_person_medications

      # 2. Preload takes using the specific IDs we just found
      preload_takes

      # 3. Aggregate all doses
      routine_tasks = sort_rows(aggregate_family_doses)
      as_needed_items = sort_rows(aggregate_family_as_needed_items)

      # 4. Sort by time and return
      Result.new(
        routine_tasks: routine_tasks,
        routine_tasks_by_person: group_rows_by_person(routine_tasks),
        as_needed_by_person: group_rows_by_person(as_needed_items)
      )
    end

    def fetch_active_schedules
      schedules_by_person_id = Schedule.active
                                       .where(person_id: @person_ids)
                                       .includes(:medication)
                                       .to_a
                                       .group_by(&:person_id)

      @people.to_h { |person| [person.id, schedules_by_person_id.fetch(person.id, [])] }
    end

    def fetch_person_medications
      person_medications_by_person_id = PersonMedication.where(person_id: @person_ids)
                                                        .includes(:medication)
                                                        .to_a
                                                        .group_by(&:person_id)

      @people.to_h { |person| [person.id, person_medications_by_person_id.fetch(person.id, [])] }
    end

    def preload_takes
      # Fetch takes for the last 30 days to cover weekly/monthly cycles
      # but focus on today for the dashboard
      all_takes = fetch_takes_for_sources

      # Group by [source_type, source_id] for fast lookup
      takes_by_source = all_takes.group_by do |t|
        t.schedule_id ? ['Schedule', t.schedule_id] : ['PersonMedication', t.person_medication_id]
      end

      # Associate preloaded takes with these objects to avoid N+1 in TimingRestrictions
      associate_takes_to_sources(all_sources, takes_by_source)
    end

    def all_sources
      @all_schedules.values.flatten + @all_person_medications.values.flatten
    end

    def fetch_takes_for_sources
      schedule_ids = @all_schedules.values.flatten.map(&:id)
      pm_ids = @all_person_medications.values.flatten.map(&:id)
      range = 30.days.ago..Time.current.end_of_day

      MedicationTake.where(taken_at: range, schedule_id: schedule_ids)
                    .or(MedicationTake.where(taken_at: range, person_medication_id: pm_ids))
                    .includes(:taken_from_location, :taken_from_medication)
                    .to_a
    end

    def associate_takes_to_sources(sources, takes_by_source)
      sources.each do |source|
        key = [source.class.name, source.id]
        takes = takes_by_source[key] || []

        # Set the association as loaded and assign the takes
        association = source.association(:medication_takes)
        association.loaded!
        association.target.concat(takes)
      end
    end

    def aggregate_family_doses
      aggregate_rows { |source, member| generate_doses_for(source, member) }
    end

    def aggregate_family_as_needed_items
      aggregate_rows { |source, member| generate_as_needed_rows_for(source, member) }
    end

    def aggregate_rows
      @people.each_with_object([]) do |member, rows|
        sources_for(member).each { |source| rows.concat(yield(source, member)) }
      end
    end

    def sources_for(member)
      (@all_schedules[member.id] || []) + (@all_person_medications[member.id] || [])
    end

    def generate_doses_for(source, person)
      return [] if as_needed_source?(source)

      # 1. Get doses already taken today from our preloaded association
      # This uses the association preloaded in aggregate_family_doses to avoid N+1 queries
      takes = todays_takes(source)
      doses = generate_taken_doses(takes, source, person)

      # 2. Determine if an upcoming dose should be shown
      # We show the "next available" dose if it falls within today
      doses << build_upcoming_row(source, person, takes) if upcoming_routine_row?(source)
      doses
    end

    def todays_takes(source) = source.medication_takes.select { |take| Time.current.all_day.cover?(take.taken_at) }

    def upcoming_routine_row?(source)
      expected_doses = expected_routine_doses_for(source)
      expected_doses.positive? && taken_count_for_cycle(source, Time.current) < expected_doses
    end

    def build_upcoming_row(source, person, takes)
      {
        person: person,
        source: source,
        scheduled_at: routine_scheduled_at(source, takes.length),
        taken_at: nil,
        status: MedicationStockSourceResolver.new(user: current_user, source: source).blocked_reason || :upcoming
      }.merge(dose_progress_for(takes, expected_routine_doses_for(source)))
    end

    def generate_as_needed_rows_for(source, person)
      return [] unless as_needed_source?(source)

      status = as_needed_status_for(source)
      takes = todays_takes(source)
      [{
        person: person,
        source: source,
        scheduled_at: as_needed_scheduled_at(source, status),
        taken_at: nil,
        status: status
      }.merge(dose_progress_for(takes, daily_dose_limit_for(source)))]
    end

    def generate_taken_doses(takes, source, person)
      takes.map do |take|
        {
          person: person,
          source: source,
          scheduled_at: take.taken_at,
          taken_at: take.taken_at,
          status: :taken,
          taken_from_location_name: take.inventory_location&.name
        }.merge(dose_progress_for(takes, expected_routine_doses_for(source)))
      end
    end

    def dose_progress_for(takes, limit)
      { daily_dose_count: takes.size, daily_dose_limit: limit, today_takes: takes.sort_by(&:taken_at) }
    end

    def expected_routine_doses_for(source)
      source.is_a?(Schedule) ? expected_schedule_doses_for(source) : source.max_daily_doses.presence || 1
    end

    def expected_schedule_doses_for(schedule)
      return 0 unless schedule.applies_on?(Date.current)

      expected = schedule.expected_doses_on(Date.current)
      return expected unless expected == 1 && schedule.effective_max_daily_doses.blank?
      return expected if schedule.effective_min_hours_between_doses.blank?

      (24 / schedule.effective_min_hours_between_doses.to_f).ceil
    end

    def taken_count_for_cycle(source, now)
      cycle = source_cycle(source)
      source.medication_takes.count { |take| cycle.range_for(now).cover?(take.taken_at) }
    end

    def as_needed_status_for(source)
      blocked_reason = MedicationStockSourceResolver.new(user: current_user, source: source).blocked_reason
      return :available if blocked_reason.blank?
      return :max_reached if blocked_reason == :cooldown && daily_limit_reached?(source)

      blocked_reason
    end

    def as_needed_scheduled_at(source, status)
      return Time.current if status == :available
      return source.next_available_time if %i[cooldown max_reached].include?(status)

      nil
    end

    def daily_limit_reached?(source)
      source.dose_constraints.would_exceed_daily_limit?(
        takes: source.medication_takes.to_a,
        cycle: source_cycle(source),
        check_time: Time.current
      )
    end

    def daily_dose_limit_for(source)
      source.is_a?(Schedule) ? source.effective_max_daily_doses(Date.current) : source.max_daily_doses
    end

    def source_cycle(source) = DoseCycle.new(source.respond_to?(:dose_cycle) ? source.dose_cycle : 'daily')

    def as_needed_source?(source) = source.is_a?(Schedule) ? schedule_as_needed?(source) : source.as_needed?

    def schedule_as_needed?(schedule)
      schedule.schedule_type_prn? || schedule_config_as_needed?(schedule) || frequency_as_needed?(schedule)
    end

    def schedule_config_as_needed?(schedule) = schedule.schedule_config.to_h['as_needed'] == true

    def frequency_as_needed?(schedule) = schedule.frequency.to_s.casecmp('as needed').zero?

    def routine_scheduled_at(source, taken_count)
      return unless source.is_a?(Schedule)

      configured_time_at(source, taken_count)
    end

    def configured_time_at(schedule, index)
      raw_time = Array(schedule.schedule_config.to_h['times']).compact_blank[index]
      return if raw_time.blank?

      current_day_at(*configured_hour_and_minute(raw_time))
    end

    def configured_hour_and_minute(raw_time)
      raw_time.to_s.split(':').map(&:to_i)
    end

    def current_day_at(hour, minute)
      date = Date.current
      Time.zone.local(date.year, date.month, date.day, hour, minute)
    end

    def sort_rows(rows)
      rows.sort_by { |row| [row[:scheduled_at] || Time.current.end_of_day, row[:source].id] }
    end

    def group_rows_by_person(rows)
      grouped = @people.index_with { [] }
      rows.each { |row| grouped[row[:person]] << row }
      grouped
    end
  end
end
