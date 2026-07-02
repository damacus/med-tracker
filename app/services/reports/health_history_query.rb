# frozen_string_literal: true

module Reports
  class HealthHistoryQuery
    Result = Data.define(:people, :medication_takes, :suspected_side_effects, :notable_illnesses, :illness_patterns)
    MedicationTakeEntry = Data.define(:person, :taken_at, :medication_name, :dose_amount, :dose_unit, :source_type,
                                      :location_name) do
      def dose_display = [dose_amount, dose_unit].compact.join(' ')
    end
    HealthEventEntry = Data.define(:event, :medication_names) do
      delegate :person, :event_kind, :title, :started_on, :ended_on, :severity, :notes, :action_taken,
               :medical_help_sought, :ongoing?, to: :event

      def title = event.title.to_s.strip

      def duration_days
        return if ongoing?

        (ended_on - started_on).to_i + 1
      end
    end

    attr_reader :people, :start_date, :end_date

    def initialize(people:, start_date:, end_date:)
      @people = people
      @start_date = start_date
      @end_date = end_date
    end

    def call
      Result.new(
        people: people_records,
        medication_takes: medication_take_entries,
        suspected_side_effects: health_event_entries(health_events.suspected_side_effect),
        notable_illnesses: health_event_entries(health_events.illness),
        illness_patterns: HealthEvents::PatternSummary.new(events: health_events.illness).call
      )
    end

    private

    def people_records
      @people_records ||= Person.where(id: person_ids).order(:name, :id).to_a
    end

    def person_ids
      @person_ids ||= if people.respond_to?(:pluck)
                        people.pluck(:id)
                      else
                        Array(people).map(&:id)
                      end
    end

    def medication_take_entries
      medication_takes.map do |take|
        source = take.source
        MedicationTakeEntry.new(
          person: take.person,
          taken_at: take.taken_at,
          medication_name: medication_display_name(source&.medication || take.taken_from_medication),
          dose_amount: take.dose_amount,
          dose_unit: take.dose_unit,
          source_type: source_type(source),
          location_name: take.inventory_location&.name
        )
      end
    end

    def medication_takes
      @medication_takes ||= medication_take_scope
                            .where(schedule_id: schedule_ids)
                            .or(medication_take_scope.where(person_medication_id: person_medication_ids))
                            .order(:taken_at, :id)
    end

    def medication_take_scope
      MedicationTake.includes(
        :taken_from_medication,
        :taken_from_location,
        schedule: %i[person medication],
        person_medication: %i[person medication]
      ).where(taken_at: date_range)
    end

    def schedule_ids
      @schedule_ids ||= Schedule.where(person_id: person_ids).select(:id)
    end

    def person_medication_ids
      @person_medication_ids ||= PersonMedication.where(person_id: person_ids).select(:id)
    end

    def date_range
      start_date.in_time_zone.beginning_of_day..end_date.in_time_zone.end_of_day
    end

    def source_type(source)
      return :as_needed if source.is_a?(PersonMedication)
      return :as_needed if source.respond_to?(:schedule_type_prn?) && source.schedule_type_prn?

      :scheduled
    end

    def medication_display_name(medication)
      medication&.friendly_name.presence || medication&.name
    end

    def health_event_entries(scope)
      scope.order(:started_on, :id).map do |event|
        HealthEventEntry.new(
          event: event,
          medication_names: event.health_event_medications.map(&:medication_name)
        )
      end
    end

    def health_events
      @health_events ||= HealthEvent
                         .includes(:person, :health_event_medications)
                         .where(person_id: person_ids)
                         .where(started_on: ..end_date)
                         .where(ended_on: nil)
                         .or(
                           HealthEvent
                             .includes(:person, :health_event_medications)
                             .where(person_id: person_ids)
                             .where(started_on: ..end_date, ended_on: start_date..)
                         )
    end
  end
end
