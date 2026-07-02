# frozen_string_literal: true

module Api
  module V1
    class SchedulesController < BaseController
      def index
        authorize Schedule
        render_collection(policy_scope(Schedule), serializer: ScheduleSerializer, includes: %i[person medication])
      end

      def show
        schedule = policy_scope(Schedule).includes(:person, :medication).find(params.expect(:id))
        authorize schedule

        render_resource(schedule, serializer: ScheduleSerializer)
      end

      def create
        attributes = schedule_params
        person = policy_scope(Person).find(attributes.delete(:person_id))
        authorize person, :show?
        assert_medication_access!(attributes[:medication_id])
        schedule = person.schedules.build(attributes)
        authorize schedule

        return render_validation_errors(schedule) unless schedule.save

        render_resource(schedule.reload, serializer: ScheduleSerializer, status: :created)
      end

      def update
        schedule = policy_scope(Schedule).includes(:person, :medication).find(params.expect(:id))
        authorize schedule
        attributes = schedule_update_params
        assert_medication_access!(attributes[:medication_id]) if attributes[:medication_id].present?

        return render_validation_errors(schedule) unless schedule.update(attributes)

        render_resource(schedule.reload, serializer: ScheduleSerializer)
      end

      private

      def schedule_params
        params.expect(
          schedule: [
            :person_id,
            :medication_id,
            :dose_amount,
            :dose_unit,
            :source_dosage_option_id,
            :frequency,
            :start_date,
            :end_date,
            :notes,
            :max_daily_doses,
            :min_hours_between_doses,
            :dose_cycle,
            :schedule_type,
            :schedule_config,
            { schedule_config: {} }
          ]
        )
      end

      def schedule_update_params
        schedule_params.except(:person_id)
      end

      def assert_medication_access!(medication_id)
        policy_scope(Medication).find(medication_id)
      end
    end
  end
end
