# frozen_string_literal: true

module Api
  module V1
    module Auth
      class SessionsController < ActionController::API
        include ErrorRendering

        def create
          account = Account.find_by(email: params.expect(:email).to_s.strip.downcase)
          household_membership = requested_household_membership(account)
          password = params.expect(:password).to_s

          unless login_permitted?(account, household_membership, password)
            render_invalid_credentials
            return
          end

          api_session, access_token, refresh_token = ApiSession.issue_for(
            account: account,
            household_membership: household_membership,
            device_name: params[:device_name],
            user_agent: request.user_agent,
            audit_context: audit_context(account)
          )

          render json: { data: login_payload(api_session, access_token, refresh_token, household_membership) },
                 status: :created
        end

        def refresh
          api_session = ApiSession.lookup_by_refresh_token(params.expect(:refresh_token).to_s)

          token_payload = nil
          TenantContext.with(account: api_session&.account, household: nil, request_id: request.request_id) do
            unless refresh_permitted?(api_session)
              record_expired_session(api_session)
              render_invalid_refresh_token
              next
            end

            access_token, refresh_token = api_session.rotate_tokens!(audit_context: audit_context(api_session.account))
            token_payload = refresh_payload(api_session, access_token, refresh_token)
          end
          return if performed?

          render json: { data: token_payload }
        end

        def destroy
          token = request.headers['Authorization'].to_s.split(' ', 2).last
          api_credential = ApiSession.lookup_by_access_token(token) || ApiAppToken.lookup_by_token(token)
          api_credential&.revoke!(audit_context: audit_context(api_credential.account))

          head :no_content
        end

        private

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

        def record_expired_session(api_session)
          return unless api_session&.refresh_expires_at&.past?

          AuthTokenAuditLogger.new.record(
            account: api_session.account,
            token_type: 'api_session',
            action: 'expired',
            metadata: {
              device_name: api_session.device_name,
              user_agent: api_session.user_agent,
              expires_at: api_session.refresh_expires_at
            },
            context: audit_context(api_session.account).merge(
              household_id: api_session.household_membership&.household_id,
              actor_membership_id: api_session.household_membership_id
            ).compact
          )
        end

        def login_permitted?(account, household_membership, password)
          return false unless account&.verified?
          return false if ApiAuthState.locked_out?(account)

          unless ApiAuthState.password_authenticated?(account, password)
            ApiLoginFailureRecorder.record_failure(account)
            return false
          end

          ApiLoginFailureRecorder.clear_failures(account)
          !ApiAuthState.mfa_configured?(account) &&
            account.person&.user&.active? &&
            household_membership.present?
        end

        def refresh_permitted?(api_session)
          api_session&.active_refresh_token? &&
            api_session.account.verified? &&
            api_session.account.person&.user&.active? &&
            !ApiAuthState.locked_out?(api_session.account)
        end

        def household_requested?
          params[:household_id].present?
        end

        def requested_household_membership(account)
          return if account.blank?

          return membership_for_requested_household(account) if household_requested?

          sole_active_membership(account)
        end

        def membership_for_requested_household(account)
          household = Household.find_by(id: params.expect(:household_id))
          return unless household

          TenantContext.with(account: account, household: household, request_id: request.request_id) do
            HouseholdMembership.active.find_by(account: account, household: household)
          end
        end

        def sole_active_membership(account)
          TenantContext.with(account: account, household: nil, request_id: request.request_id) do
            memberships = HouseholdMembership.active.where(account: account).limit(2).to_a
            memberships.first if memberships.one?
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

        def login_payload(api_session, access_token, refresh_token, household_membership)
          refresh_payload(api_session, access_token, refresh_token).merge(
            me: Api::V1::MeSerializer.new(api_session.account.person.user).as_json,
            household: household_payload(household_membership&.household)
          )
        end

        def refresh_payload(api_session, access_token, refresh_token)
          {
            access_token: access_token,
            access_token_expires_at: api_session.access_expires_at.iso8601,
            refresh_token: refresh_token,
            refresh_token_expires_at: api_session.refresh_expires_at.iso8601,
            household: household_payload(api_session.household_membership&.household)
          }
        end

        def render_invalid_credentials
          render_api_error(
            code: 'invalid_credentials',
            message: 'Email or password is invalid',
            status: :unauthorized
          )
        end

        def render_invalid_refresh_token
          render_api_error(
            code: 'invalid_refresh_token',
            message: 'Refresh token is invalid or expired',
            status: :unauthorized
          )
        end
      end
    end
  end
end
