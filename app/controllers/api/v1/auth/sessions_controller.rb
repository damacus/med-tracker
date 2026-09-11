# frozen_string_literal: true

module Api
  module V1
    module Auth
      class SessionsController < ActionController::API
        include ErrorRendering

        def index
          api_credential = authenticated_api_credential
          return render_authentication_required unless api_credential

          render json: {
            data: sessions_for(api_credential).order(created_at: :desc).map do |session|
              session_payload(session)
            end
          }
        end

        def households
          api_credential = authenticated_api_credential
          return render_authentication_required unless api_credential

          memberships = operational_memberships(api_credential.account)
          render json: { data: memberships.map do |membership|
            household_payload(membership.household).merge(role: membership.role, membership_id: membership.id)
          end }
        end

        def revoke
          api_credential = authenticated_api_credential
          return render_authentication_required unless api_credential

          api_session = sessions_for(api_credential).find(params.expect(:id))
          api_session.revoke!(audit_context: audit_context(api_credential.account), action: 'revoked')

          head :no_content
        end

        def destroy
          token = request.headers['Authorization'].to_s.split(' ', 2).last
          api_credential = ApiSession.lookup_by_access_token(token) || ApiAppToken.lookup_by_token(token) ||
                           OauthGrant.lookup_by_access_token(token)
          api_credential&.revoke!(audit_context: audit_context(api_credential.account))

          head :no_content
        end

        private

        def sessions_for(credential)
          return credential.account.api_sessions.active unless credential.is_a?(OauthGrant) && credential.mobile?

          grants = OauthGrant.mobile.where(account_id: credential.account_id, revoked_at: nil)
                             .where.not(token_hash: nil)
                             .where('last_used_at > ?', AuthenticationLifetime.inactivity_days.days.ago)
          maximum_age = AuthenticationLifetime.maximum_age_days
          grants = grants.where('authenticated_at > ?', maximum_age.days.ago) if maximum_age.positive?
          grants
        end

        def audit_context(account)
          {
            whodunnit: audit_user_id(account),
            ip: request.remote_ip,
            request_id: request.request_id
          }
        end

        def audit_user_id(account)
          person = account&.person
          person&.user&.id
        end

        def operational_memberships(account)
          TenantContext.with(account: account, household: nil, request_id: request.request_id) do
            account.household_memberships.active.joins(:household).merge(Household.operational)
                   .includes(:household).order(:id).to_a
          end
        end

        def household_payload(household)
          return nil unless household

          {
            id: household.id,
            slug: household.slug,
            name: household.name
          }
        end

        def session_payload(api_session)
          return mobile_session_payload(api_session) if api_session.is_a?(OauthGrant)

          {
            id: api_session.id,
            device_name: api_session.device_name,
            household_id: api_session.household_membership&.household_id,
            last_used_at: api_session.last_used_at.iso8601,
            access_token_expires_at: api_session.access_expires_at.iso8601,
            refresh_token_expires_at: api_session.refresh_expires_at.iso8601,
            created_at: api_session.created_at.iso8601
          }
        end

        def mobile_session_payload(grant)
          { id: grant.id, device_name: grant.device_name, last_used_at: grant.last_used_at.iso8601,
            access_token_expires_at: grant.expires_in.iso8601,
            refresh_token_expires_at: grant.refresh_expires_at.iso8601,
            created_at: grant.created_at.iso8601 }
        end

        def render_authentication_required
          render_api_error(
            code: 'unauthorized',
            message: 'Authentication required',
            status: :unauthorized
          )
        end

        def authenticated_api_credential
          token = request.headers['Authorization'].to_s.split(' ', 2).last
          credential = ApiSession.lookup_by_access_token(token) || ApiAppToken.lookup_by_token(token) ||
                       OauthGrant.lookup_by_access_token(token)
          return unless valid_account_credential?(credential)
          return if credential.is_a?(ApiSession) && !credential.access_expires_at.future?
          return if ApiAuthState.locked_out?(credential.account)

          credential.touch_last_used!
          credential
        end

        def valid_account_credential?(credential)
          return credential.mobile? && credential.active_for_account? if credential.is_a?(OauthGrant)

          credential&.active_for_membership?
        end
      end
    end
  end
end
