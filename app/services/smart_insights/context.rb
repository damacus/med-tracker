# frozen_string_literal: true

module SmartInsights
  class Context
    delegate :daily_data, :inventory_alerts, to: :report_data

    attr_reader :people, :start_date, :end_date

    def initialize(people:, start_date:, end_date:)
      @people = people
      @start_date = start_date
      @end_date = end_date
    end

    def schedules
      @schedules ||= Schedule.where(person_id: person_ids)
                             .where('start_date <= ? AND end_date >= ?', end_date, start_date)
                             .includes(:person, :medication)
                             .to_a
    end

    def active_schedules
      @active_schedules ||= schedules.select(&:active?)
    end

    def person_medications
      @person_medications ||= PersonMedication.where(person_id: person_ids)
                                              .includes(:person, :medication)
                                              .to_a
    end

    def takes
      @takes ||= source_scope.where(taken_at: start_date.beginning_of_day..end_date.end_of_day)
                             .includes(:schedule, :person_medication)
                             .to_a
    end

    def evidence_days
      (start_date..end_date).count
    end

    def expected_events
      daily_data.sum { |day| day[:expected] }
    end

    def logged_events
      daily_data.sum { |day| day[:actual] } + prn_takes.count
    end

    def enough_evidence?
      evidence_days >= IndexQuery::MINIMUM_EVIDENCE_DAYS &&
        (expected_events + logged_events) >= IndexQuery::MINIMUM_EVENTS
    end

    def evidence_summary
      I18n.t(
        'smart_insights.evidence_summary',
        count: expected_events + logged_events,
        days: evidence_days,
        events: expected_events + logged_events
      )
    end

    def prn_takes
      takes.select { |take| prn_source?(take.source) }
    end

    def prn_sources
      schedules.select(&:schedule_type_prn?) +
        person_medications.select(&:as_needed?)
    end

    private

    def person_ids
      @person_ids ||= if people.respond_to?(:pluck)
                        people.pluck(:id)
                      else
                        Array(people).map(&:id)
                      end
    end

    def report_data
      @report_data ||= Reports::IndexQuery.new(people: people, start_date: start_date, end_date: end_date).call
    end

    def source_scope
      schedule_ids = schedules.map(&:id)
      person_medication_ids = person_medications.map(&:id)

      MedicationTake.where(schedule_id: schedule_ids)
                    .or(MedicationTake.where(person_medication_id: person_medication_ids))
    end

    def prn_source?(source)
      return false if source.blank?
      return source.schedule_type_prn? if source.is_a?(Schedule)

      source.as_needed?
    end
  end
end
