# frozen_string_literal: true

module MedTrackerMcp
  class Context
    class Error < StandardError; end

    class AuthenticationError < Error
      attr_reader :status, :code

      def initialize(message = 'Authentication required')
        @status = :unauthorized
        @code = 'unauthorized'
        super
      end
    end

    attr_reader :request

    class << self
      def resolve!(request)
        new(request).tap(&:authenticate!)
      end
    end

    def initialize(request)
      @request = request
    end

    def authenticate!
      raise AuthenticationError unless valid_session?
      raise AuthenticationError unless valid_account_and_user?
      raise AuthenticationError if ApiAuthState.locked_out?(account)
      raise AuthenticationError unless active_membership?

      api_credential.touch_last_used!
      self
    end

    def to_h
      {
        account: account,
        account_id: account.id,
        household: household,
        household_id: household.id,
        membership: membership,
        membership_id: membership.id,
        api_credential: api_credential,
        request_id: request.request_id,
        remote_ip: request.remote_ip
      }
    end

    def with_current(&)
      TenantContext.with(**tenant_context) do
        bind_audit_context
        yield
      end
    ensure
      Audit::Context.clear!
      Current.reset
    end

    private

    def tenant_context
      {
        account: account,
        household: household,
        membership: membership,
        request_id: request.request_id
      }
    end

    def bind_audit_context
      Audit::Context.start!(
        request: request,
        account: account,
        user: user,
        membership: membership,
        credential: api_credential
      )
    end

    def valid_session?
      api_credential.present? && !revoked? && unexpired?
    end

    def valid_account_and_user?
      account.present? && account.verified? && user.present? && user.active?
    end

    def active_membership?
      api_credential.active_for_membership?
    end

    def revoked?
      api_credential.revoked_at.present?
    end

    def unexpired?
      return api_credential.access_expires_at.future? if api_credential.is_a?(ApiSession)

      api_credential.is_a?(ApiAppToken)
    end

    def api_credential
      @api_credential ||= begin
        token = bearer_token
        ApiSession.lookup_by_access_token(token) || ApiAppToken.lookup_by_token(token)
      end
    end

    def account
      api_credential.account
    end

    def household
      membership.household
    end

    def membership
      api_credential.household_membership
    end

    def user
      account.person&.user
    end

    def bearer_token
      scheme, token = request.headers['Authorization'].to_s.split(' ', 2)
      return token if scheme == 'Bearer'

      nil
    end
  end
end
