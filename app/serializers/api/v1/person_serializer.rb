# frozen_string_literal: true

module Api
  module V1
    class PersonSerializer
      def initialize(person)
        @person = person
      end

      def as_json(*)
        identity_data.merge(relationship_data)
      end

      private

      attr_reader :person

      def identity_data
        {
          id: person.id,
          name: person.name,
          email: person.email,
          date_of_birth: person.date_of_birth&.iso8601,
          person_type: person.person_type,
          has_capacity: person.has_capacity,
          updated_at: person.updated_at.iso8601
        }
      end

      def relationship_data
        {
          age: person.age,
          location_ids: person.locations.map(&:id),
          notification_preference_id: person.notification_preference&.id
        }
      end
    end
  end
end
