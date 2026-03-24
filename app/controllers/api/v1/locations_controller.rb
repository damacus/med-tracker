# frozen_string_literal: true

module Api
  module V1
    class LocationsController < BaseController
      def index
        authorize Location
        render_collection(policy_scope(Location), serializer: LocationSerializer)
      end

      def show
        location = policy_scope(Location).find(params[:id])
        authorize location

        render_resource(location, serializer: LocationSerializer)
      end
    end
  end
end
