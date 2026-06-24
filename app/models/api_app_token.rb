# frozen_string_literal: true

class ApiAppToken < ApplicationRecord
  TOKEN_PREFIX = 'mt_app_'
  LAST_USED_TOUCH_INTERVAL = 5.minutes

  belongs_to :account
  belongs_to :household_membership

  validates :name, :token_digest, :last_used_at, presence: true
  validates :token_digest, uniqueness: true
  validate :household_membership_must_belong_to_account

  scope :active, -> { where(revoked_at: nil) }

  class << self
    def issue_for(account:, household_membership:, name:, audit_context: nil)
      raw_token = build_token
      app_token = create!(
        account: account,
        household_membership: household_membership,
        permissions_version: household_membership.permissions_version,
        name: name,
        token_digest: digest(raw_token),
        last_used_at: Time.current
      )

      record_audit(app_token, 'created', audit_context)

      [app_token, raw_token]
    end

    def lookup_by_token(token)
      active.find_by(token_digest: digest(token))
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

    revoked_at.nil? && household_membership.active? && permissions_version == household_membership.permissions_version
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
