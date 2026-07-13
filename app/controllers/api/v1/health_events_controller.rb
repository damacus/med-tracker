# frozen_string_literal: true

module Api
  module V1
    class HealthEventsController < BaseController
      def index
        authorize HealthEvent
        render_collection(policy_scope(HealthEvent), serializer: HealthEventSerializer, includes: %i[person medications])
      end

      def show
        health_event = find_api_record(policy_scope(HealthEvent).includes(:person, :medications), params.expect(:id))
        authorize health_event

        render_resource(health_event, serializer: HealthEventSerializer)
      end

      def create
        health_event = HealthEvent.new(health_event_params)
        health_event.household = current_household
        authorize health_event

        return render_validation_errors(health_event) unless health_event.save

        render_resource(health_event.reload, serializer: HealthEventSerializer, status: :created)
      end

      def update
        health_event = find_api_record(policy_scope(HealthEvent).includes(:person, :medications), params.expect(:id))
        authorize health_event
        return unless fresh_api_record?(health_event)

        return render_validation_errors(health_event) unless health_event.update(health_event_update_params)

        render_resource(health_event.reload, serializer: HealthEventSerializer)
      end

      private

      def health_event_params
        attributes = params.expect(
          health_event: [
            :person_id,
            :event_kind,
            :severity,
            :title,
            :notes,
            :started_on,
            :ended_on,
            { medication_ids: [] }
          ]
        )
        if attributes[:person_id].present?
          attributes[:person_id] = api_record_id(policy_scope(Person), attributes[:person_id])
        end
        attributes[:medication_ids] = Array(attributes[:medication_ids]).map do |identifier|
          api_record_id(policy_scope(Medication), identifier)
        end
        attributes
      end

      def health_event_update_params
        health_event_params.except(:person_id)
      end
    end
  end
end
