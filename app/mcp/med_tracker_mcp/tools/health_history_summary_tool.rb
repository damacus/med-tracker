# frozen_string_literal: true

module MedTrackerMcp
  module Tools
    class HealthHistorySummaryTool < BaseTool
      MAX_RANGE_DAYS = 180

      tool_name 'medtracker_health_history_summary'
      description 'Return a bounded health-history summary for visible people.'
      input_schema(
        properties: {
          start_date: { type: 'string', description: 'Inclusive ISO8601 start date.' },
          end_date: { type: 'string', description: 'Inclusive ISO8601 end date.' },
          person_ids: { type: 'array', items: { type: 'integer' } }
        },
        required: []
      )

      class << self
        def call(server_context:, start_date: nil, end_date: nil, person_ids: nil)
          dates = date_range(start_date, end_date)
          return error_response('Health history date range cannot exceed 180 days.') if range_too_large?(dates)

          context = tool_context(server_context)
          context.with_current do
            people = visible_people(context, person_ids)
            result = Reports::HealthHistoryQuery.new(
              people: people,
              start_date: dates.fetch(:start_date),
              end_date: dates.fetch(:end_date)
            ).call

            response(payload(result, dates), 'Bounded MedTracker health-history summary.')
          end
        rescue ArgumentError
          error_response('start_date and end_date must be valid ISO8601 dates.')
        end

        private

        def date_range(start_date, end_date)
          end_on = end_date.present? ? Date.iso8601(end_date.to_s) : Time.zone.today
          start_on = start_date.present? ? Date.iso8601(start_date.to_s) : end_on - 30.days

          { start_date: start_on, end_date: end_on }
        end

        def range_too_large?(dates)
          (dates.fetch(:end_date) - dates.fetch(:start_date)).to_i > MAX_RANGE_DAYS
        end

        def visible_people(context, person_ids)
          scope = context.policy_scope(Person).includes(:locations, :notification_preference).order(:name, :id)
          person_ids.present? ? scope.where(id: person_ids) : scope
        end

        def payload(result, dates)
          {
            format: 'medtracker.mcp.health_history_summary.v1',
            start_date: dates.fetch(:start_date).iso8601,
            end_date: dates.fetch(:end_date).iso8601,
            people: people_payloads(result),
            medication_takes: medication_take_payloads(result),
            suspected_side_effects: suspected_side_effect_payloads(result),
            notable_illnesses: notable_illness_payloads(result),
            illness_patterns: result.illness_patterns
          }
        end

        def people_payloads(result)
          result.people.map { |person| Api::V1::PersonSerializer.new(person).as_json }
        end

        def medication_take_payloads(result)
          result.medication_takes.map { |entry| medication_take_payload(entry) }
        end

        def suspected_side_effect_payloads(result)
          result.suspected_side_effects.map { |entry| health_event_payload(entry) }
        end

        def notable_illness_payloads(result)
          result.notable_illnesses.map { |entry| health_event_payload(entry) }
        end

        def medication_take_payload(entry)
          {
            person_id: entry.person.id,
            person_name: entry.person.name,
            taken_at: entry.taken_at.iso8601,
            medication_name: entry.medication_name,
            dose: entry.dose_display,
            source_type: entry.source_type,
            location_name: entry.location_name
          }
        end

        def health_event_payload(entry)
          {
            person_id: entry.person.id,
            person_name: entry.person.name,
            event_kind: entry.event_kind,
            title: entry.title,
            started_on: entry.started_on&.iso8601,
            ended_on: entry.ended_on&.iso8601,
            severity: entry.severity,
            medication_names: entry.medication_names
          }
        end
      end
    end
  end
end
