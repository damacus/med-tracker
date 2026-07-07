# frozen_string_literal: true

module Api
  class OidcSessionExchange
    class Error < StandardError; end

    Result = Data.define(:api_session, :access_token, :refresh_token, :household_membership)

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
      membership = membership_for(account)
      validate_account!(account)
      validate_membership!(membership)

      result_for(account, membership, claims)
    end

    private

    attr_reader :params, :request

    def result_for(account, membership, claims)
      api_session, access_token, refresh_token = issue_session(account, membership, claims)
      Result.new(api_session: api_session, access_token: access_token, refresh_token: refresh_token,
                 household_membership: membership)
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

    def membership_for(account)
      scope = account.household_memberships.active.includes(:household).order(:id)
      return scope.find_by(household_id: params[:household_id]) if params[:household_id].present?

      memberships = scope.limit(2).to_a
      memberships.first if memberships.one?
    end

    def validate_account!(account)
      raise Error, 'OIDC account is unavailable' unless account&.verified? && account.person&.user&.active?
      raise Error, 'OIDC account is unavailable' if ApiAuthState.locked_out?(account)
    end

    def validate_membership!(membership)
      raise Error, 'OIDC household membership is unavailable' unless membership&.active?
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
  end
end
