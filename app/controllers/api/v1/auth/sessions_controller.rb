# frozen_string_literal: true

module Api
  module V1
    module Auth
      class SessionsController < ActionController::API
        def create
          account = Account.find_by(email: params.expect(:email).to_s.strip.downcase)

          unless authenticated_account?(account, params.expect(:password).to_s)
            render_invalid_credentials
            return
          end

          user = account.person&.user
          unless user&.active?
            render_invalid_credentials
            return
          end

          api_session, access_token, refresh_token = ApiSession.issue_for(
            account: account,
            device_name: params[:device_name],
            user_agent: request.user_agent,
            audit_context: audit_context(account)
          )

          render json: {
            data: {
              access_token: access_token,
              access_token_expires_at: api_session.access_expires_at.iso8601,
              refresh_token: refresh_token,
              refresh_token_expires_at: api_session.refresh_expires_at.iso8601,
              me: Api::V1::MeSerializer.new(user).as_json
            }
          }, status: :created
        end

        def refresh
          api_session = ApiSession.lookup_by_refresh_token(params.expect(:refresh_token).to_s)
          unless api_session&.active_refresh_token? && api_session.account.verified? && api_session.account.person&.user&.active?
            record_expired_session(api_session)
            render_invalid_refresh_token
            return
          end

          access_token, refresh_token = api_session.rotate_tokens!(audit_context: audit_context(api_session.account))

          render json: {
            data: {
              access_token: access_token,
              access_token_expires_at: api_session.access_expires_at.iso8601,
              refresh_token: refresh_token,
              refresh_token_expires_at: api_session.refresh_expires_at.iso8601
            }
          }
        end

        def destroy
          token = request.headers['Authorization'].to_s.split(' ', 2).last
          api_session = ApiSession.lookup_by_access_token(token)
          api_session&.revoke!(audit_context: audit_context(api_session.account))

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
            context: audit_context(api_session.account)
          )
        end

        def authenticated_account?(account, password)
          return false if account.blank? || password.blank?
          return false unless account.verified?

          BCrypt::Password.new(account.password_hash).is_password?(password)
        rescue BCrypt::Errors::InvalidHash
          false
        end

        def render_invalid_credentials
          render json: { error: { code: 'invalid_credentials', message: 'Email or password is invalid' } },
                 status: :unauthorized
        end

        def render_invalid_refresh_token
          render json: { error: { code: 'invalid_refresh_token', message: 'Refresh token is invalid or expired' } },
                 status: :unauthorized
        end
      end
    end
  end
end
