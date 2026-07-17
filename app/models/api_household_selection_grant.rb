# frozen_string_literal: true

class ApiHouseholdSelectionGrant < ApplicationRecord
  class InvalidGrant < StandardError; end

  TOKEN_TTL = 5.minutes
  TOKEN_PREFIX = 'mt_household_'

  Result = Data.define(:api_session, :access_token, :refresh_token, :household_membership)

  belongs_to :account

  validates :token_digest, :expires_at, presence: true
  validates :token_digest, uniqueness: true
  validates :oidc_mfa_verified, inclusion: { in: [true, false] }
  validate :expires_at_must_be_in_future, on: :create

  class << self
    def issue_for(account:, **attributes)
      token = "#{TOKEN_PREFIX}#{SecureRandom.urlsafe_base64(48)}"
      grant = create!(
        account: account,
        token_digest: digest(token),
        expires_at: TOKEN_TTL.from_now,
        **attributes
      )
      [grant, token]
    end

    def select_household(token:, household_id:, audit_context: nil)
      transaction do
        grant = usable_grant!(token)
        membership = operational_membership(grant.account, household_id) || raise(InvalidGrant)
        grant.update!(used_at: Time.current)
        issue_session(grant, membership, audit_context)
      end
    end

    def digest(token)
      Digest::SHA256.hexdigest(token.to_s)
    end

    def account_available?(account)
      account&.verified? && account.person&.user&.active? && !ApiAuthState.locked_out?(account)
    end

    private

    def usable_grant!(token)
      grant = lock.find_by(token_digest: digest(token))
      raise InvalidGrant unless grant&.usable? && account_available?(grant.account)

      grant
    end

    def issue_session(grant, membership, audit_context)
      api_session, access_token, refresh_token = ApiSession.issue_for(
        account: grant.account,
        household_membership: membership,
        **session_attributes(grant, audit_context)
      )
      Result.new(api_session:, access_token:, refresh_token:, household_membership: membership)
    end

    def session_attributes(grant, audit_context)
      {
        device_name: grant.device_name,
        user_agent: grant.user_agent,
        mfa_verified_at: grant.mfa_verified_at,
        oidc_mfa_verified: grant.oidc_mfa_verified,
        audit_context: audit_context.to_h.merge(whodunnit: grant.account.person&.user&.id)
      }
    end

    def operational_membership(account, household_id)
      account.household_memberships.active.joins(:household).merge(Household.operational)
             .find_by(household_id: household_id)
    end
  end

  def usable?
    used_at.nil? && expires_at.future?
  end

  private

  def expires_at_must_be_in_future
    return if expires_at.blank? || expires_at.future?

    errors.add(:expires_at, 'must be in the future')
  end
end
