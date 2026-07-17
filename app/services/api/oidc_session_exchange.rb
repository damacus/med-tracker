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

    def initialize(params:, request:, provider_client: OidcProviderClient.new)
      @params = params
      @request = request
      @provider_client = provider_client
    end

    def call
      validate_required_params!
      claims = verified_claims
      validate_claims!(claims)
      account = account_for(claims)

      TenantContext.with(account: account, household: nil, request_id: request.request_id) do
        validate_account!(account)
        memberships = operational_memberships(account).to_a
        raise Error if memberships.empty?

        reject_replay!(claims)
        if memberships.one?
          session_result(account, memberships.sole, claims)
        else
          selection_result(account, memberships, claims)
        end
      end
    rescue OidcProviderClient::Error, ActiveRecord::RecordInvalid
      raise Error
    end

    private

    attr_reader :params, :request, :provider_client

    def verified_claims
      id_token = provider_client.exchange_code(
        authorization_code: params[:authorization_code],
        code_verifier: params[:code_verifier],
        redirect_uri: params[:redirect_uri]
      )
      provider_client.decode_id_token(id_token)
    end

    def validate_required_params!
      required = %i[authorization_code code_verifier redirect_uri nonce]
      raise Error if required.any? { params[it].blank? }
    end

    def validate_claims!(claims)
      raise Error unless valid_nonce?(claims['nonce'])
      raise Error if claims['sub'].blank?
      raise Error unless valid_issued_at?(claims['iat'])
    rescue ArgumentError, KeyError
      raise Error
    end

    def valid_nonce?(claim)
      nonce = claim.to_s
      nonce.present? && ActiveSupport::SecurityUtils.secure_compare(nonce, params[:nonce].to_s)
    end

    def valid_issued_at?(claim)
      Time.zone.at(Integer(claim)) <= 1.minute.from_now
    end

    def reject_replay!(claims)
      ApiOidcNonce.create!(
        issuer: claims.fetch('iss'),
        subject: claims.fetch('sub'),
        nonce: claims.fetch('nonce'),
        used_at: Time.current
      )
    rescue ActiveRecord::RecordNotUnique, ActiveRecord::RecordInvalid
      raise Error
    end

    def account_for(claims)
      AccountIdentity.find_by!(provider: 'oidc', uid: claims.fetch('sub')).account
    rescue ActiveRecord::RecordNotFound
      raise Error
    end

    def validate_account!(account)
      raise Error unless ApiHouseholdSelectionGrant.account_available?(account)
    end

    def operational_memberships(account)
      account.household_memberships.active.joins(:household).merge(Household.operational)
             .includes(:household).order(:id)
    end

    def session_result(account, membership, claims)
      TenantContext.with(
        account: account,
        household: membership.household,
        membership: membership,
        request_id: request.request_id
      ) do
        api_session, access_token, refresh_token = ApiSession.issue_for(
          account: account,
          household_membership: membership,
          device_name: params[:device_name],
          user_agent: request.user_agent,
          **mfa_attributes(claims),
          audit_context: audit_context(account, membership)
        )
        Result.new(
          api_session: api_session,
          access_token: access_token,
          refresh_token: refresh_token,
          household_membership: membership
        )
      end
    end

    def selection_result(account, memberships, claims)
      grant, token = ApiHouseholdSelectionGrant.issue_for(
        account: account,
        device_name: params[:device_name],
        user_agent: request.user_agent,
        **mfa_attributes(claims)
      )
      Result.new(selection_grant: grant, selection_token: token, household_memberships: memberships)
    end

    def mfa_attributes(claims)
      verified = Array(claims['amr']).map(&:to_s).intersect?(ApiAuthState::MFA_METHODS) ||
                 claims['acr'].to_s.include?('mfa')
      {
        mfa_verified_at: verified ? Time.current : nil,
        oidc_mfa_verified: verified
      }
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
  end
end
