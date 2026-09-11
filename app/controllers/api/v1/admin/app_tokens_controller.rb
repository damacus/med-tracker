# frozen_string_literal: true

module Api
  module V1
    module Admin
      class AppTokensController < BaseController
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
            audit_context: audit_context,
            **app_token_params.slice(:expires_at).to_h.symbolize_keys
          )

          render json: { data: token_payload(app_token).merge(token: raw_token) }, status: :created
        rescue ActiveRecord::RecordInvalid => e
          render_validation_errors(e.record)
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
          params.expect(api_app_token: %i[name expires_at])
        end

        def token_payload(token)
          {
            id: token.id,
            name: token.name,
            last_used_at: token.last_used_at&.iso8601,
            expires_at: token.expires_at.iso8601,
            revoked_at: token.revoked_at&.iso8601,
            permissions_version: token.permissions_version
          }
        end
      end
    end
  end
end
