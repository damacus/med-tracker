# frozen_string_literal: true

module Profiles
  class ApiTokensController < ApplicationController
    before_action :require_authentication
    before_action :check_two_factor_setup

    def create
      authorize current_user.person, :update?
      if ApiAuthState.locked_out?(current_account)
        return redirect_to(profile_path, alert: t('profiles.api_tokens.locked'))
      end
      return redirect_to(profile_path, alert: t('profiles.api_tokens.mfa_required')) unless mfa_satisfied?

      membership = token_household_membership
      _app_token, raw_token = ApiAppToken.issue_for(
        account: current_account,
        household_membership: membership,
        name: token_params.fetch(:name).to_s.strip,
        audit_context: audit_context(membership)
      )

      render Views::Profiles::Show.new(
        person: current_user.person,
        account: current_account,
        new_api_app_token: raw_token,
        api_app_tokens: current_account.api_app_tokens.active.order(created_at: :desc).to_a
      ), status: :created
    rescue ActiveRecord::RecordInvalid => e
      redirect_to profile_path, alert: e.record.errors.full_messages.to_sentence
    end

    def destroy
      authorize current_user.person, :update?
      app_token = current_account.api_app_tokens.find(params.expect(:id))
      app_token.revoke!(audit_context: audit_context(app_token.household_membership))

      redirect_to profile_path, notice: t('profiles.api_tokens.revoked')
    end

    private

    def token_params
      params.expect(api_app_token: %i[name household_membership_id])
    end

    def token_household_membership
      current_account.household_memberships.active.find(token_params.fetch(:household_membership_id))
    end

    def mfa_satisfied?
      ApiAuthState.web_session_mfa_satisfied?(session, current_account)
    end

    def audit_context(membership)
      {
        whodunnit: current_user.id,
        ip: request.remote_ip,
        request_id: request.request_id,
        household_id: membership.household_id,
        actor_membership_id: membership.id
      }
    end
  end
end
