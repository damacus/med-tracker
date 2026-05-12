# frozen_string_literal: true

class ApiSession < ApplicationRecord
  ACCESS_TOKEN_TTL = 15.minutes
  REFRESH_TOKEN_TTL = 30.days

  belongs_to :account

  validates :access_token_digest, :refresh_token_digest, :access_expires_at,
            :refresh_expires_at, :last_used_at, presence: true

  scope :active, -> { where(revoked_at: nil) }

  class << self
    def issue_for(account:, device_name: nil, user_agent: nil, audit_context: nil)
      access_token = build_token
      refresh_token = build_token
      session = create_session(account:, device_name:, user_agent:, access_token:, refresh_token:)

      record_audit(session, 'created', audit_context)

      [session, access_token, refresh_token]
    end

    def lookup_by_access_token(token)
      active.find_by(access_token_digest: digest(token))
    end

    def lookup_by_refresh_token(token)
      active.find_by(refresh_token_digest: digest(token))
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
        context: audit_context
      )
    end

    private

    def create_session(account:, device_name:, user_agent:, access_token:, refresh_token:)
      now = Time.current

      create!(
        account: account,
        device_name: device_name,
        user_agent: user_agent,
        last_used_at: now,
        access_expires_at: now + ACCESS_TOKEN_TTL,
        refresh_expires_at: now + REFRESH_TOKEN_TTL,
        access_token_digest: digest(access_token),
        refresh_token_digest: digest(refresh_token)
      )
    end

    def build_token
      "mt_#{SecureRandom.urlsafe_base64(48)}"
    end

    def audit_logger
      AuthTokenAuditLogger.new
    end
  end

  def active_refresh_token?
    revoked_at.nil? && refresh_expires_at.future?
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
    update!(last_used_at: Time.current)
  end

  private

  def audit_metadata
    {
      device_name: device_name,
      user_agent: user_agent,
      expires_at: refresh_expires_at
    }
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
end
