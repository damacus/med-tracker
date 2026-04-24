# frozen_string_literal: true

module Reports
  class IndexQuery
    Result = Data.define(:daily_data, :inventory_alerts)

    attr_reader :people, :start_date, :end_date

    def initialize(people:, start_date:, end_date:)
      @people = people
      @start_date = start_date
      @end_date = end_date
    end

    def call
      Result.new(
        daily_data: daily_data,
        inventory_alerts: inventory_alerts
      )
    end

    private

    def daily_data
      (start_date..end_date).map { |date| daily_row_for(date) }
    end

    def inventory_alerts
      alerts = Schedule.active.where(person_id: person_ids)
                       .includes(:medication)
                       .map { |schedule| inventory_alert_for(schedule) }
                       .compact

      alerts.select { |alert| alert[:days_left] < 14 }
            .sort_by { |alert| alert[:days_left] }
            .take(2)
    end

    def schedules
      @schedules ||= Schedule.where(person_id: person_ids)
                             .where('start_date <= ? AND (end_date IS NULL OR end_date >= ?)', end_date, start_date)
                             .to_a
    end

    def takes_by_date
      @takes_by_date ||= MedicationTake.where(schedule_id: schedules.map(&:id))
                                       .where(taken_at: start_date.beginning_of_day..end_date.end_of_day)
                                       .group_by { |take| take.taken_at.to_date }
    end

    def person_ids
      @person_ids ||= if people.respond_to?(:pluck)
                        people.pluck(:id)
                      else
                        Array(people).map(&:id)
                      end
    end

    def daily_row_for(date)
      expected_doses = expected_doses_for(date)
      actual_doses = takes_by_date[date]&.size || 0

      {
        date: date,
        day_name: date.strftime('%a'),
        percentage: compliance_percentage(expected_doses:, actual_doses:),
        expected: expected_doses,
        actual: actual_doses
      }
    end

    def expected_doses_for(date)
      schedules_for(date).sum { |schedule| schedule.expected_doses_on(date) }
    end

    def schedules_for(date)
      schedules.select do |schedule|
        schedule.start_date <= date && (schedule.end_date.nil? || schedule.end_date >= date)
      end
    end

    def inventory_alert_for(schedule)
      burn_rate = inventory_burn_rate_for(schedule)
      return if burn_rate <= 0

      current = schedule.medication.current_supply || 0
      days_left = (current.to_f / burn_rate).floor

      {
        medication_name: schedule.medication.name,
        days_left: days_left,
        doses_left: current,
        low_stock: days_left <= 3
      }
    end

    def inventory_burn_rate_for(schedule)
      dates = inventory_projection_dates_for(schedule)
      return 0 if dates.empty?

      dates.sum { |date| schedule.expected_doses_on(date) }.to_f / dates.size
    end

    def inventory_projection_dates_for(schedule)
      projection_start = [Time.zone.today, schedule.start_date].max
      projection_end = [Time.zone.today + 30.days, schedule.end_date].compact.min
      return [] if projection_end < projection_start

      (projection_start..projection_end).to_a
    end

    def compliance_percentage(expected_doses:, actual_doses:)
      return 100 if expected_doses.zero?

      [(actual_doses.to_f / expected_doses * 100).round, 100].min
    end
  end
end
