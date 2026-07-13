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
          result = access_change.create_grant(
            grant_params.to_h.merge(
              household: current_household,
              granted_by_membership: current_membership
            )
          )
          grant = result.record
          return render_validation_errors(grant) unless result.success?

          render json: { data: grant_payload(grant) }, status: :created
        end

        def destroy
          grant = PersonAccessGrant.where(household: current_household).find(params.expect(:id))
          if grant.carer_relationship
            grant.errors.add(:base, 'Relationship-owned grants must be revoked through their carer relationship')
            return render_validation_errors(grant)
          end

          result = access_change.revoke_grant(grant)
          return render_validation_errors(result.record) unless result.success?

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

        def access_change
          @access_change ||= Households::AccessChange.new(
            actor_account: current_account,
            actor_membership: current_membership,
            request: request
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
