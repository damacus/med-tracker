# frozen_string_literal: true

module Api
  module V1
    class LocationsController < BaseController
      def index
        authorize Location
        render_collection(policy_scope(Location), serializer: LocationSerializer)
      end

      def show
        location = find_api_record(policy_scope(Location), params.expect(:id))
        authorize location

        render_resource(location, serializer: LocationSerializer)
      end

      def create
        record = Location.new(location_params)
        record.household = current_household
        return render_validation_errors(record) unless record.save

        render_resource(record, serializer: LocationSerializer, status: :created)
      end

      def update
        location.with_lock do
          next unless fresh_location?
          next render_validation_errors(location) unless location.update(location_params)

          render_resource(location, serializer: LocationSerializer)
        end
      end

      def destroy
        location.with_lock do
          next unless fresh_location?
          if MedicationAdministrationHistory.exists_for?(location)
            next render_unprocessable('Location cannot be deleted while administration history exists')
          end
          next render_validation_errors(location) unless location.destroy

          head :no_content
        end
      end

      private

      def with_api_idempotency(&)
        case action_name
        when 'create' then authorize Location, :create?
        when 'update' then authorize location, :update?
        when 'destroy' then authorize location, :destroy?
        end
        super
      end

      def location
        @location ||= find_api_record(policy_scope(Location), params.expect(:id))
      end

      def location_params
        params.expect(location: %i[name description])
      end

      def fresh_location?
        if request.headers['If-Match'].blank?
          render_api_error(code: 'precondition_required', message: 'A current location version is required',
                           status: :precondition_required)
          return false
        end

        fresh_api_record?(location)
      end
    end
  end
end
