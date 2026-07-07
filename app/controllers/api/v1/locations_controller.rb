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
    end
  end
end
