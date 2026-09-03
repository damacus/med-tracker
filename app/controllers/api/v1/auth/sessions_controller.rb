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
            data: api_credential.account.api_sessions.active.order(created_at: :desc).map do |session|
              session_payload(session)
            end
          }
        end

        def create
          account = Account.find_by(email: params.expect(:email).to_s.strip.downcase)
          password = params.expect(:password).to_s

          unless login_permitted?(account, password)
            render_invalid_credentials
            return
          end

          memberships = operational_memberships(account)
          if household_selection_required?(memberships)
            selection_grant, selection_token = ApiHouseholdSelectionGrant.issue_for(
              account: account,
              device_name: params[:device_name],
              user_agent: request.user_agent
            )
            render json: {
              data: household_selection_payload(
                selection_grant: selection_grant,
                selection_token: selection_token,
                memberships: memberships
              )
            }, status: :accepted
            return
          end

          household_membership = selected_membership(memberships)
          unless household_membership
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

        def oidc_exchange
          result = Api::OidcSessionExchange.new(params: oidc_exchange_params, request: request).call
          if result.household_selection_required?
            render json: {
              data: household_selection_payload(
                selection_grant: result.selection_grant,
                selection_token: result.selection_token,
                memberships: result.household_memberships
              )
            }, status: :accepted
            return
          end

          render json: {
            data: login_payload(result.api_session, result.access_token, result.refresh_token,
                                result.household_membership)
          }, status: :created
        rescue Api::OidcSessionExchange::Error
          render_invalid_oidc_exchange
        end

        def select_household
          result = ApiHouseholdSelectionGrant.select_household(
            token: params.expect(:selection_token).to_s,
            household_id: params.expect(:household_id),
            audit_context: audit_context(nil)
          )
          render json: {
            data: login_payload(result.api_session, result.access_token, result.refresh_token,
                                result.household_membership)
          }, status: :created
        rescue ApiHouseholdSelectionGrant::InvalidGrant
          render_invalid_household_selection
        end

        def households
          api_credential = authenticated_api_credential
          return render_authentication_required unless api_credential

          render json: { data: household_memberships_payload(api_credential.account) }
        end

        def revoke
          api_credential = authenticated_api_credential
          return render_authentication_required unless api_credential

          api_session = api_credential.account.api_sessions.active.find(params.expect(:id))
          api_session.revoke!(audit_context: audit_context(api_credential.account), action: 'revoked')

          head :no_content
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

        def oidc_exchange_params
          params.permit(:id_token, :nonce, :code_verifier, :device_name, :household_id, :provider)
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

        def login_permitted?(account, password)
          login_account_available?(account) &&
            password_login_permitted?(account, password) &&
            !ApiAuthState.mfa_configured?(account) &&
            account.person&.user&.active?
        end

        def login_account_available?(account)
          account&.verified? && !ApiAuthState.locked_out?(account)
        end

        def password_login_permitted?(account, password)
          if ApiAuthState.password_authenticated?(account, password)
            ApiLoginFailureRecorder.clear_failures(account)
            return true
          end

          ApiLoginFailureRecorder.record_failure(account)
          false
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

        def household_selection_required?(memberships)
          !household_requested? && memberships.many?
        end

        def selected_membership(memberships)
          return memberships.first unless household_requested?

          household_id = params.expect(:household_id).to_s
          memberships.find { |membership| membership.household_id.to_s == household_id }
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

        def household_memberships_payload(account)
          account.household_memberships.active.joins(:household).merge(Household.operational)
                 .includes(:household).order(:id).map do |membership|
            household_payload(membership.household).merge(
              role: membership.role,
              membership_id: membership.id
            )
          end
        end

        def household_selection_payload(selection_grant:, selection_token:, memberships:)
          {
            status: 'household_selection_required',
            selection_token: selection_token,
            selection_expires_at: selection_grant.expires_at.iso8601,
            households: memberships.map do |membership|
              household_payload(membership.household).merge(
                role: membership.role,
                membership_id: membership.id
              )
            end
          }
        end

        def session_payload(api_session)
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

        def render_invalid_oidc_exchange
          render_api_error(
            code: 'invalid_oidc_exchange',
            message: 'OIDC exchange is invalid',
            status: :unauthorized
          )
        end

        def render_invalid_household_selection
          render_api_error(
            code: 'invalid_household_selection',
            message: 'Household selection is invalid or expired',
            status: :unauthorized
          )
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
          credential = ApiSession.lookup_by_access_token(token) || ApiAppToken.lookup_by_token(token)
          return unless credential&.active_for_membership?
          return if credential.is_a?(ApiSession) && !credential.access_expires_at.future?
          return if ApiAuthState.locked_out?(credential.account)

          credential.touch_last_used!
          credential
        end
      end
    end
  end
end
