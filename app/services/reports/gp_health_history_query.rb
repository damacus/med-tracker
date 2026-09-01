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

    def initialize(person:, start_date:, end_date:)
      @person = person
      @start_date = start_date
      @end_date = end_date
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
        medication_takes: [],
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
