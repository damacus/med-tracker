# frozen_string_literal: true

module Api
  module V1
    module Admin
      class MembershipsController < BaseController
        before_action :require_fresh_privileged_action, only: %i[update destroy]

        def index
          memberships = current_household.household_memberships.includes(:account, :person).order(:id)
          render json: { data: memberships.map { |membership| membership_payload(membership) } }
        end

        def update
          membership = find_membership
          return render_validation_errors(membership) unless membership.update(membership_params)

          audit_admin_action!(event_type: 'api/admin/membership/updated', target: membership, outcome: 'success')
          render json: { data: membership_payload(membership.reload) }
        end

        def destroy
          membership = find_membership
          membership.update!(status: :revoked, revoked_at: Time.current,
                             permissions_version: membership.permissions_version + 1)
          audit_admin_action!(event_type: 'api/admin/membership/revoked', target: membership, outcome: 'success')
          head :no_content
        end

        private

        def find_membership
          current_household.household_memberships.find(params.expect(:id))
        end

        def membership_params
          params.expect(household_membership: %i[role status person_id])
        end

        def membership_payload(membership)
          {
            id: membership.id,
            account_id: membership.account_id,
            email: membership.account.email,
            person_id: membership.person_id,
            person_name: membership.person&.name,
            role: membership.role,
            status: membership.status,
            permissions_version: membership.permissions_version,
            joined_at: membership.joined_at&.iso8601
          }
        end
      end
    end
  end
end
