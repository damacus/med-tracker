module Api
  module V1
    class LocationMembershipsController < BaseController
      rescue_from ActiveRecord::RecordInvalid, with: :render_invalid_membership

      def create
        location.with_lock do
          record = location.location_memberships.find_or_create_by!(person: person)
          render json: { data: LocationMembershipSerializer.new(record).as_json }, status: :created
        end
      end

      def destroy
        membership.destroy!
        head :no_content
      end

      private

      def with_api_idempotency(&)
        authorize location, :update?
        authorize(action_name == 'create' ? LocationMembership : membership, "#{action_name}?")
        authorize person, :update?
        super
      end

      def location
        @location ||= find_api_record(policy_scope(Location), params.expect(:location_id))
      end

      def membership
        @membership ||= location.location_memberships.find(params.expect(:id))
      end

      def person
        @person ||= if action_name == 'create'
                      identifier = params.expect(location_membership: [:person_id]).fetch(:person_id)
                      find_api_record(policy_scope(Person), identifier)
                    else
                      policy_scope(Person).find(membership.person_id)
                    end
      end

      def render_invalid_membership
        render_unprocessable('Location membership could not be saved')
      end
    end
  end
end
