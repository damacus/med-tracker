# frozen_string_literal: true

module FamilyDashboard
  # Query object to fetch a 24-hour medication schedule for a person and their dependents
  class ScheduleQuery
    Result = Data.define(:routine_tasks, :routine_tasks_by_person, :as_needed_by_person, :today_takes_by_person)

    delegate :routine_tasks, :routine_tasks_by_person, :as_needed_by_person, :today_takes_by_person, to: :result

    attr_reader :current_user, :date, :now

    def initialize(people, current_user: nil, date: Date.current, now: Time.current, include_paused: false)
      @people = Array(people)
      @current_user = current_user
      @date = date
      @now = now
      @include_paused = include_paused
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
      preload_stock_states

      # 3. Aggregate all doses
      routine_tasks = sort_rows(aggregate_family_doses)
      as_needed_items = sort_rows(aggregate_family_as_needed_items)
      today_takes = build_today_takes_by_person

      # 4. Sort by time and return
      Result.new(
        routine_tasks: routine_tasks,
        routine_tasks_by_person: group_rows_by_person(routine_tasks),
        as_needed_by_person: group_rows_by_person(as_needed_items),
        today_takes_by_person: today_takes
      )
    end

    def fetch_active_schedules
      schedules = Schedule.current.where(start_date: ..date, end_date: date..)
      schedules = schedules.where(active: true) unless @include_paused
      schedules_by_person_id = schedules.where(person_id: @person_ids)
                                        .includes(:medication)
                                        .to_a
                                        .group_by(&:person_id)

      @people.to_h { |person| [person.id, schedules_by_person_id.fetch(person.id, [])] }
    end

    def fetch_person_medications
      person_medications = PersonMedication.current
      person_medications = person_medications.where(active: true) unless @include_paused
      person_medications_by_person_id = person_medications.where(person_id: @person_ids)
                                                          .includes(:medication)
                                                          .to_a
                                                          .group_by(&:person_id)

      @people.to_h { |person| [person.id, person_medications_by_person_id.fetch(person.id, [])] }
    end

    def preload_takes
      # Fetch takes for the last 30 days to cover weekly/monthly cycles
      # but focus on today for the dashboard
      TakePreloader.new(sources: all_sources, date: date).call
    end

    def all_sources
      @all_schedules.values.flatten + @all_person_medications.values.flatten
    end

    def aggregate_family_doses
      aggregate_rows { |source, member| generate_doses_for(source, member) }
    end

    def aggregate_family_as_needed_items
      aggregate_rows { |source, member| generate_as_needed_rows_for(source, member) }
    end

    def build_today_takes_by_person
      @people.index_with { |member| sort_takes(sources_for(member).flat_map { |source| todays_takes(source) }) }
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

      # 1. Get today's takes from our preloaded association for progress and history
      # This uses the association preloaded in aggregate_family_doses to avoid N+1 queries
      takes = todays_takes(source)

      # 2. Determine if an upcoming dose should be shown
      # We show only the next actionable routine row if it falls within today
      return [] unless upcoming_routine_row?(source)

      [build_upcoming_row(source, person, takes)]
    end

    def todays_takes(source) = source.medication_takes.select { |take| date.all_day.cover?(take.taken_at) }

    def upcoming_routine_row?(source)
      expected_doses = expected_routine_doses_for(source)
      expected_doses.positive? && taken_count_for_cycle(source, now) < expected_doses
    end

    def build_upcoming_row(source, person, takes)
      stock_state = stock_state_for(source)
      {
        person: person,
        source: source,
        scheduled_at: routine_scheduled_at(source, takes.length),
        taken_at: nil,
        status: stock_state[:status] || :upcoming,
        can_record: stock_state[:can_record],
        stock_source_choices: stock_state[:choices]
      }.merge(dose_progress_for(takes, expected_routine_doses_for(source)))
    end

    def generate_as_needed_rows_for(source, person)
      return [] unless as_needed_source?(source)

      stock_state = stock_state_for(source)
      status = as_needed_status_for(source, stock_state)
      takes = todays_takes(source)
      [{
        person: person,
        source: source,
        scheduled_at: as_needed_scheduled_at(source, status),
        taken_at: nil,
        status: status,
        can_record: stock_state[:can_record] && status == :available,
        stock_source_choices: stock_state[:choices]
      }.merge(dose_progress_for(takes, daily_dose_limit_for(source)))]
    end

    def dose_progress_for(takes, limit)
      { daily_dose_count: takes.size, daily_dose_limit: limit, today_takes: takes.sort_by(&:taken_at) }
    end

    def expected_routine_doses_for(source)
      source.is_a?(Schedule) ? expected_schedule_doses_for(source) : source.max_daily_doses.presence || 1
    end

    def expected_schedule_doses_for(schedule)
      return 0 unless schedule.applies_on?(date)

      expected = schedule.expected_doses_on(date)
      return expected unless expected == 1 && schedule.effective_max_daily_doses.blank?
      return expected if schedule.effective_min_hours_between_doses.blank?

      (24 / schedule.effective_min_hours_between_doses.to_f).ceil
    end

    def taken_count_for_cycle(source, now)
      cycle = source_cycle(source)
      source.medication_takes.count { |take| cycle.range_for(now).cover?(take.taken_at) }
    end

    def as_needed_status_for(source, stock_state)
      blocked_reason = stock_state[:status]
      return :available if blocked_reason.blank?
      return :max_reached if blocked_reason == :cooldown && daily_limit_reached?(source)

      blocked_reason
    end

    def stock_state_for(source)
      @stock_state_loader.state_for(source)
    end

    def preload_stock_states
      @stock_state_loader = StockStateLoader.new(
        sources: all_sources,
        person_ids: @person_ids,
        current_user: current_user,
        date: date,
        now: now
      ).call
    end

    def as_needed_scheduled_at(source, status)
      return now if status == :available
      return next_available_time_for(source) if %i[cooldown max_reached].include?(status)

      nil
    end

    def daily_limit_reached?(source)
      @stock_state_loader.daily_limit_reached?(source)
    end

    def next_available_time_for(source)
      @stock_state_loader.next_available_time_for(source)
    end

    def daily_dose_limit_for(source)
      source.is_a?(Schedule) ? source.effective_max_daily_doses(date) : source.max_daily_doses
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
      Time.zone.local(date.year, date.month, date.day, hour, minute)
    end

    def sort_rows(rows)
      rows.sort_by { |row| [row[:scheduled_at] || date.end_of_day, row[:source].id] }
    end

    def sort_takes(takes) = takes.uniq { |take| dose_history_key(take) }.sort_by(&:taken_at)

    def dose_history_key(take)
      return [take.class.name, take.id] if take.is_a?(ApplicationRecord) && take.id.present?

      take.object_id
    end

    def group_rows_by_person(rows)
      grouped = @people.index_with { [] }
      rows.each { |row| grouped[row[:person]] << row }
      grouped
    end
  end
end
