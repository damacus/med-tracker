# frozen_string_literal: true

module Api
  module V1
    module Admin
      class PersonAccessGrantsController < BaseController
        before_action :require_fresh_privileged_action, only: %i[create destroy]

        def index
          grants = PersonAccessGrant.where(household: current_household).includes(:person, :household_membership)
                                    .order(:id)
          render json: { data: grants.map { |grant| grant_payload(grant) } }
        end

        def create
          grant = PersonAccessGrant.new(grant_params)
          grant.household = current_household
          grant.granted_by_membership = current_membership

          return render_validation_errors(grant) unless grant.save

          audit_admin_action!(event_type: 'api/admin/person_access_grant/created', target: grant, outcome: 'success')
          render json: { data: grant_payload(grant) }, status: :created
        end

        def destroy
          grant = PersonAccessGrant.where(household: current_household).find(params.expect(:id))
          grant.update!(revoked_at: Time.current)
          audit_admin_action!(event_type: 'api/admin/person_access_grant/revoked', target: grant, outcome: 'success')
          head :no_content
        end

        private

        def grant_params
          params.expect(
            person_access_grant: %i[
              household_membership_id
              person_id
              access_level
              relationship_type
              expires_at
            ]
          )
        end

        def grant_payload(grant)
          {
            id: grant.id,
            household_membership_id: grant.household_membership_id,
            person_id: grant.person_id,
            person_name: grant.person.name,
            access_level: grant.access_level,
            relationship_type: grant.relationship_type,
            expires_at: grant.expires_at&.iso8601,
            revoked_at: grant.revoked_at&.iso8601
          }
        end
      end
    end
  end
end
