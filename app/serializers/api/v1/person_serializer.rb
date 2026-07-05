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
        portable_identity_data.merge(profile_data)
      end

      def portable_identity_data
        {
          id: person.id,
          portable_id: person.portable_id,
          updated_at: person.updated_at.iso8601
        }
      end

      def profile_data
        {
          name: person.name,
          email: person.email,
          date_of_birth: person.date_of_birth&.iso8601,
          person_type: person.person_type,
          has_capacity: person.has_capacity
        }
      end

      def relationship_data
        {
          age: person.age,
          location_ids: person.locations.map(&:id),
          location_portable_ids: person.locations.map(&:portable_id),
          notification_preference_id: person.notification_preference&.id,
          notification_preference_portable_id: person.notification_preference&.portable_id
        }
      end
    end
  end
end
