# frozen_string_literal: true

module Api
  class OidcSessionExchange
    class Error < StandardError; end

    Result = Struct.new(
      :api_session,
      :access_token,
      :refresh_token,
      :household_membership,
      :selection_grant,
      :selection_token,
      :household_memberships,
      keyword_init: true
    ) do
      def household_selection_required? = selection_grant.present?
    end

    def initialize(params:, request:)
      @params = params
      @request = request
    end

    def call
      validate_pkce!
      claims = decode_claims
      validate_nonce!(claims)
      reject_replay!(claims)
      account = account_for(claims)
      validate_account!(account)
      memberships = operational_memberships(account)
      membership = selected_membership(memberships)

      return session_result(account, membership, claims) if membership
      return selection_result(account, memberships, claims) if !household_requested? && memberships.many?

      raise Error, 'OIDC household membership is unavailable'
    end

    private

    attr_reader :params, :request

    def session_result(account, membership, claims)
      api_session, access_token, refresh_token = issue_session(account, membership, claims)
      Result.new(
        api_session: api_session,
        access_token: access_token,
        refresh_token: refresh_token,
        household_membership: membership
      )
    end

    def selection_result(account, memberships, claims)
      selection_grant, selection_token = ApiHouseholdSelectionGrant.issue_for(
        account: account,
        device_name: params[:device_name],
        user_agent: request.user_agent,
        **mfa_attributes(claims)
      )
      Result.new(selection_grant: selection_grant, selection_token: selection_token,
                 household_memberships: memberships)
    end

    def issue_session(account, membership, claims)
      mfa_verified = oidc_mfa_verified?(claims)
      ApiSession.issue_for(
        account: account,
        household_membership: membership,
        device_name: params[:device_name],
        user_agent: request.user_agent,
        mfa_verified_at: mfa_verified ? Time.current : nil,
        oidc_mfa_verified: mfa_verified,
        audit_context: audit_context(account, membership)
      )
    end

    def validate_pkce!
      raise Error, 'PKCE verifier is required' if params[:code_verifier].blank?
      raise Error, 'OIDC token is required' if params[:id_token].blank?
    end

    def decode_claims
      payload, = JWT.decode(
        params[:id_token].to_s,
        client_secret,
        true,
        algorithm: 'HS256',
        iss: issuer,
        verify_iss: true,
        aud: audience,
        verify_aud: true,
        verify_expiration: true
      )
      payload
    rescue JWT::DecodeError => e
      raise Error, e.message
    end

    def validate_nonce!(claims)
      raise Error, 'OIDC nonce is invalid' if claims['nonce'].blank? || claims['nonce'] != params[:nonce].to_s
      raise Error, 'OIDC subject is invalid' if claims['sub'].blank?
    end

    def reject_replay!(claims)
      ApiOidcNonce.create!(
        issuer: claims.fetch('iss'),
        subject: claims.fetch('sub'),
        nonce: claims.fetch('nonce'),
        used_at: Time.current
      )
    rescue ActiveRecord::RecordNotUnique, ActiveRecord::RecordInvalid
      raise Error, 'OIDC nonce has already been used'
    end

    def account_for(claims)
      AccountIdentity.find_by!(provider: params.fetch(:provider, 'oidc'), uid: claims.fetch('sub')).account
    rescue ActiveRecord::RecordNotFound
      raise Error, 'OIDC identity is not linked'
    end

    def operational_memberships(account)
      TenantContext.with(account: account, household: nil, request_id: request.request_id) do
        account.household_memberships.active.joins(:household).merge(Household.operational)
               .includes(:household).order(:id).to_a
      end
    end

    def selected_membership(memberships)
      return memberships.first if !household_requested? && memberships.one?
      return unless household_requested?

      household_id = params[:household_id].to_s
      memberships.find { |membership| membership.household_id.to_s == household_id }
    end

    def household_requested?
      params[:household_id].present?
    end

    def validate_account!(account)
      raise Error, 'OIDC account is unavailable' unless account&.verified? && account.person&.user&.active?
      raise Error, 'OIDC account is unavailable' if ApiAuthState.locked_out?(account)
    end

    def issuer
      ENV.fetch('OIDC_ISSUER_URL', nil).presence || Rails.application.credentials.dig(:oidc, :issuer_url).to_s
    end

    def audience
      ENV.fetch('OIDC_MOBILE_CLIENT_ID', nil).presence ||
        ENV.fetch('OIDC_CLIENT_ID', nil).presence ||
        Rails.application.credentials.dig(:oidc, :client_id).to_s
    end

    def client_secret
      ENV.fetch('OIDC_CLIENT_SECRET', nil).presence ||
        Rails.application.credentials.dig(:oidc, :client_secret).to_s
    end

    def audit_context(account, membership)
      {
        whodunnit: account.person&.user&.id,
        ip: request.remote_ip,
        request_id: request.request_id,
        household_id: membership.household_id,
        actor_membership_id: membership.id
      }
    end

    def oidc_mfa_verified?(claims)
      Array(claims['amr']).map(&:to_s).intersect?(ApiAuthState::MFA_METHODS) ||
        claims['acr'].to_s.include?('mfa')
    end

    def mfa_attributes(claims)
      verified = oidc_mfa_verified?(claims)
      {
        mfa_verified_at: verified ? Time.current : nil,
        oidc_mfa_verified: verified
      }
    end
  end
end
