module Api
  module V1
    class LocationMembershipSerializer
      def initialize(membership)
        @membership = membership
      end

      def as_json(*)
        { id: @membership.id.to_s, location_id: @membership.location_id.to_s,
          location_portable_id: @membership.location.portable_id, person_id: @membership.person_id.to_s,
          person_portable_id: @membership.person.portable_id, created_at: @membership.created_at.iso8601 }
      end
    end
  end
end
