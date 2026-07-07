# frozen_string_literal: true

module MedTrackerMcp
  module Tools
    class TodayScheduleTool < BaseTool
      tool_name 'medtracker_today_schedule'
      description 'Return visible schedules and medications already taken today.'
      input_schema(properties: {}, required: [])

      class << self
        def call(server_context:)
          context = tool_context(server_context)
          context.with_current do
            response(payload(context), 'Visible MedTracker schedule context for today.')
          end
        end

        private

        def payload(context)
          {
            format: 'medtracker.mcp.today_schedule.v1',
            date: Time.zone.today.iso8601,
            schedules: schedule_payloads(context),
            taken_today: taken_today_payloads(context)
          }
        end

        def schedule_payloads(context)
          context.policy_scope(Schedule)
                 .includes(:person, :medication)
                 .order(:person_id, :id)
                 .map { |schedule| Api::V1::ScheduleSerializer.new(schedule).as_json }
        end

        def taken_today_payloads(context)
          people = context.policy_scope(Person).includes(:locations, :notification_preference)

          Reports::TodayTakenMedicationsQuery.new(people: people).call.map { |group| taken_today_payload(group) }
        end

        def taken_today_payload(group)
          {
            person_id: group.person.id,
            person_name: group.person.name,
            medications: group.medications.map(&:to_h)
          }
        end
      end
    end
  end
end
