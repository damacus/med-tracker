# frozen_string_literal: true

module Api
  module V1
    module Admin
      class AppTokensController < BaseController
        before_action :require_fresh_privileged_action, only: %i[create destroy]

        def index
          tokens = current_account.api_app_tokens
                                  .where(household_membership: current_household.household_memberships)
                                  .order(:id)
          render json: { data: tokens.map { |token| token_payload(token) } }
        end

        def create
          app_token, raw_token = ApiAppToken.issue_for(
            account: current_account,
            household_membership: current_membership,
            name: app_token_params[:name],
            audit_context: audit_context
          )

          render json: { data: token_payload(app_token).merge(token: raw_token) }, status: :created
        end

        def destroy
          token = current_account.api_app_tokens
                                 .where(household_membership: current_household.household_memberships)
                                 .find(params.expect(:id))
          token.revoke!(audit_context: audit_context)
          head :no_content
        end

        private

        def app_token_params
          params.expect(api_app_token: [:name])
        end

        def token_payload(token)
          {
            id: token.id,
            name: token.name,
            last_used_at: token.last_used_at&.iso8601,
            revoked_at: token.revoked_at&.iso8601,
            permissions_version: token.permissions_version
          }
        end
      end
    end
  end
end
