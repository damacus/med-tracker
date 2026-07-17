# frozen_string_literal: true

class ApiSession < ApplicationRecord
  ACCESS_TOKEN_TTL = 15.minutes
  REFRESH_TOKEN_TTL = 30.days
  LAST_USED_TOUCH_INTERVAL = 5.minutes

  RotationResult = Data.define(:api_session, :access_token, :refresh_token)

  belongs_to :account
  belongs_to :household_membership, optional: true

  validates :access_token_digest, :refresh_token_digest, :access_expires_at,
            :refresh_expires_at, :last_used_at, presence: true
  validates :household_membership, presence: true
  validate :household_membership_must_belong_to_account

  scope :active, -> { where(revoked_at: nil) }

  class << self
    def issue_for(account:, household_membership:, audit_context: nil, **)
      access_token = build_token
      refresh_token = build_token
      session = create_session(
        account: account,
        household_membership: household_membership,
        tokens: { access: access_token, refresh: refresh_token },
        **
      )

      record_audit(session, 'created', audit_context)

      [session, access_token, refresh_token]
    end

    def lookup_by_access_token(token)
      active.find_by(access_token_digest: digest(token))
    end

    def lookup_by_refresh_token(token)
      active.find_by(refresh_token_digest: digest(token))
    end

    def rotate_refresh_token(token, audit_context: nil)
      transaction do
        session = active.lock.find_by(refresh_token_digest: digest(token))
        return unless session&.send(:refresh_permitted?)

        access_token, refresh_token = session.send(:rotate_token_values!)
        record_audit(session, 'rotated', audit_context)
        RotationResult.new(api_session: session, access_token: access_token, refresh_token: refresh_token)
      end
    end

    def digest(token)
      Digest::SHA256.hexdigest(token.to_s)
    end

    def record_audit(session, action, audit_context)
      audit_logger.record(
        account: session.account,
        token_type: 'api_session',
        action: action,
        metadata: session.send(:audit_metadata),
        context: audit_context_with_tenant(session, audit_context)
      )
    end

    private

    def create_session(account:, household_membership:, tokens:, **options)
      now = Time.current

      create!(
        account: account,
        household_membership: household_membership,
        permissions_version: household_membership.permissions_version,
        device_name: options[:device_name],
        user_agent: options[:user_agent],
        mfa_verified_at: options[:mfa_verified_at],
        oidc_mfa_verified: options.fetch(:oidc_mfa_verified, false),
        last_used_at: now,
        access_expires_at: now + ACCESS_TOKEN_TTL,
        refresh_expires_at: now + REFRESH_TOKEN_TTL,
        access_token_digest: digest(tokens.fetch(:access)),
        refresh_token_digest: digest(tokens.fetch(:refresh))
      )
    end

    def build_token
      "mt_#{SecureRandom.urlsafe_base64(48)}"
    end

    def audit_logger
      AuthTokenAuditLogger.new
    end

    def audit_context_with_tenant(session, audit_context)
      {
        household_id: session.household_membership&.household_id,
        actor_membership_id: session.household_membership_id
      }.merge(audit_context.to_h).compact
    end
  end

  def active_refresh_token?
    revoked_at.nil? && refresh_expires_at.future? && active_for_membership?
  end

  def active_for_membership?
    return false if household_membership.blank?

    household_membership.active? && household_membership.household.operational? &&
      permissions_version == household_membership.permissions_version
  end

  def rotate_tokens!(audit_context: nil)
    access_token, refresh_token = rotate_token_values!
    self.class.record_audit(self, 'rotated', audit_context)

    [access_token, refresh_token]
  end

  def revoke!(audit_context: nil, action: 'revoked')
    update!(revoked_at: Time.current)
    self.class.record_audit(self, action, audit_context)
  end

  def touch_last_used!
    return if last_used_at.present? && last_used_at >= LAST_USED_TOUCH_INTERVAL.ago

    update!(last_used_at: Time.current)
  end

  private

  def audit_metadata
    {
      device_name: device_name,
      user_agent: user_agent,
      expires_at: refresh_expires_at,
      household_membership_id: household_membership_id,
      permissions_version: permissions_version
    }
  end

  def household_membership_must_belong_to_account
    return if household_membership.blank? || household_membership.account_id == account_id

    errors.add(:household_membership, 'must belong to the account')
  end

  def rotate_token_values!
    access_token = self.class.send(:build_token)
    refresh_token = self.class.send(:build_token)
    now = Time.current

    update!(
      access_token_digest: self.class.digest(access_token),
      refresh_token_digest: self.class.digest(refresh_token),
      access_expires_at: now + ACCESS_TOKEN_TTL,
      refresh_expires_at: now + REFRESH_TOKEN_TTL,
      last_used_at: now
    )

    [access_token, refresh_token]
  end

  def refresh_permitted?
    active_refresh_token? && account.verified? && account.person&.user&.active? && !ApiAuthState.locked_out?(account)
  end
end
