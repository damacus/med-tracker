# frozen_string_literal: true

class ApiAppToken < ApplicationRecord
  TOKEN_PREFIX = 'mt_app_'
  LAST_USED_TOUCH_INTERVAL = 5.minutes

  belongs_to :account
  belongs_to :household_membership

  validates :name, :token_digest, :last_used_at, :expires_at, presence: true
  validates :token_digest, uniqueness: true
  validate :household_membership_must_belong_to_account
  validate :expiry_within_maximum, on: :create

  scope :active, -> { where(revoked_at: nil) }

  class << self
    def issue_for(account:, household_membership:, name:, audit_context: nil,
                  expires_at: Time.current + AuthenticationLifetime.api_token_maximum_age_months.months)
      raw_token = build_token
      app_token = create!(
        account: account,
        household_membership: household_membership,
        permissions_version: household_membership.permissions_version,
        name: name,
        expires_at: expires_at,
        token_digest: digest(raw_token),
        last_used_at: Time.current
      )

      record_audit(app_token, 'created', audit_context)

      [app_token, raw_token]
    end

    def lookup_by_token(token)
      app_token = active.find_by(token_digest: digest(token))
      app_token if app_token&.unexpired?
    end

    def apply_maximum_age!
      months = AuthenticationLifetime.api_token_maximum_age_months
      where('expires_at > created_at + make_interval(months => ?)', months).find_each(&:cap_lifetime!)
    end

    def digest(token)
      Digest::SHA256.hexdigest(token.to_s)
    end

    def record_audit(app_token, action, audit_context)
      audit_logger.record(
        account: app_token.account,
        token_type: 'api_app_token',
        action: action,
        metadata: app_token.send(:audit_metadata),
        context: audit_context_with_tenant(app_token, audit_context)
      )
    end

    private

    def build_token
      "#{TOKEN_PREFIX}#{SecureRandom.urlsafe_base64(48)}"
    end

    def audit_logger
      AuthTokenAuditLogger.new
    end

    def audit_context_with_tenant(app_token, audit_context)
      {
        household_id: app_token.household_membership&.household_id,
        actor_membership_id: app_token.household_membership_id
      }.merge(audit_context.to_h).compact
    end
  end

  def active_for_membership?
    return false if household_membership.blank?

    revoked_at.nil? && unexpired? && household_membership.active? && household_membership.household&.operational? &&
      permissions_version == household_membership.permissions_version
  end

  def unexpired?
    return false unless expires_at && created_at

    maximum = created_at + AuthenticationLifetime.api_token_maximum_age_months.months
    cap_lifetime! if persisted? && expires_at > maximum
    expires_at > Time.current
  end

  def cap_lifetime!
    with_lock do
      maximum = created_at + AuthenticationLifetime.api_token_maximum_age_months.months
      update!(expires_at: maximum) if expires_at > maximum
    end
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

  def expiry_within_maximum
    return unless expires_at

    issued_at = created_at || Time.current
    maximum = issued_at + AuthenticationLifetime.api_token_maximum_age_months.months
    return if expires_at > issued_at && expires_at <= maximum

    errors.add(:expires_at, 'must be after issuance and within the configured maximum age')
  end

  def audit_metadata
    {
      device_name: name,
      household_membership_id: household_membership_id,
      permissions_version: permissions_version
    }
  end

  def household_membership_must_belong_to_account
    return if household_membership.blank? || household_membership.account_id == account_id

    errors.add(:household_membership, 'must belong to the account')
  end
end
