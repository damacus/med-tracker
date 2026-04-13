# frozen_string_literal: true

module Api
  module V1
    class SchedulesController < BaseController
      def index
        authorize Schedule
        render_collection(policy_scope(Schedule), serializer: ScheduleSerializer, includes: %i[person medication])
      end

      def show
        schedule = policy_scope(Schedule).includes(:person, :medication).find(params[:id])
        authorize schedule

        render_resource(schedule, serializer: ScheduleSerializer)
      end
    end
  end
end
