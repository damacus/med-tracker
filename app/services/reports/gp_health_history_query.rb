# frozen_string_literal: true

module Reports
  class GpHealthHistoryQuery
    Result = Data.define(
      :person,
      :current_medicines,
      :chronology,
      :people,
      :medication_takes,
      :suspected_side_effects,
      :notable_illnesses,
      :illness_patterns
    )
    HealthEventEntry = HealthHistoryQuery::HealthEventEntry

    def initialize(person:, start_date:, end_date:, include_medication_takes: false)
      @person = person
      @start_date = start_date
      @end_date = end_date
      @include_medication_takes = include_medication_takes
    end

    def call
      events = health_events
      chronology = health_event_entries(events)

      Result.new(**result_attributes(events, chronology))
    end

    private

    attr_reader :person, :start_date, :end_date

    def result_attributes(events, chronology)
      {
        person:,
        current_medicines:,
        chronology: most_recent_first(chronology),
        people: [person],
        medication_takes: medication_take_entries,
        suspected_side_effects: entries_for(chronology, :suspected_side_effect?),
        notable_illnesses: entries_for(chronology, :illness?),
        illness_patterns: HealthEvents::PatternSummary.new(events: events.select(&:illness?)).call
      }
    end

    def most_recent_first(entries) = entries.sort_by { |entry| [entry.started_on, entry.event.id] }.reverse

    def entries_for(entries, predicate)
      entries.select { |entry| entry.event.public_send(predicate) }
    end

    def current_medicines
      (scheduled_medicines + direct_medicines).uniq(&:id).sort_by do |medication|
        medication.friendly_name.to_s.downcase
      end
    end

    def scheduled_medicines
      Schedule.active.where(person:).includes(:medication).map(&:medication)
    end

    def direct_medicines
      PersonMedication.active.where(person:).includes(:medication).map(&:medication)
    end

    def health_event_entries(events)
      events.map do |event|
        HealthEventEntry.new(
          event:,
          medication_names: event.health_event_medications.map(&:medication_name)
        )
      end
    end

    def medication_take_entries
      return [] unless @include_medication_takes

      medication_takes.map do |take|
        source = take.source
        HealthHistoryQuery::MedicationTakeEntry.new(
          person:,
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
      ).where(taken_at: medication_take_range)
    end

    def medication_take_range
      start_date.in_time_zone.beginning_of_day..end_date.in_time_zone.end_of_day
    end

    def schedule_ids = Schedule.where(person:).select(:id)

    def person_medication_ids = PersonMedication.where(person:).select(:id)

    def medication_display_name(medication)
      medication&.friendly_name.presence || medication&.name
    end

    def source_type(source)
      return :as_needed if source.is_a?(PersonMedication)
      return :as_needed if source.respond_to?(:schedule_type_prn?) && source.schedule_type_prn?

      :scheduled
    end

    def health_events
      @health_events ||= HealthEvent
                         .includes(:person, health_event_medications: :medication)
                         .where(person:)
                         .where(started_on: ..end_date)
                         .where(ended_on: nil)
                         .or(
                           HealthEvent
                             .includes(:person, health_event_medications: :medication)
                             .where(person:, started_on: ..end_date, ended_on: start_date..)
                         ).to_a
    end
  end
end
