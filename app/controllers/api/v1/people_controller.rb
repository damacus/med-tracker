# frozen_string_literal: true

module Api
  module V1
    class PeopleController < BaseController
      def index
        authorize Person
        render_collection(policy_scope(Person), serializer: PersonSerializer, includes: %i[locations notification_preference])
      end

      def show
        person = policy_scope(Person).includes(:locations, :notification_preference).find(params[:id])
        authorize person

        render_resource(person, serializer: PersonSerializer)
      end
    end
  end
end
