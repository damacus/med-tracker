# frozen_string_literal: true

class ApiSession < ApplicationRecord
  ACCESS_TOKEN_TTL = 15.minutes
  REFRESH_TOKEN_TTL = 30.days

  belongs_to :account

  validates :access_token_digest, :refresh_token_digest, :access_expires_at,
            :refresh_expires_at, :last_used_at, presence: true

  scope :active, -> { where(revoked_at: nil) }

  def self.issue_for(account:, device_name: nil, user_agent: nil)
    access_token = build_token
    refresh_token = build_token
    now = Time.current

    session = create!(
      account: account,
      device_name: device_name,
      user_agent: user_agent,
      last_used_at: now,
      access_expires_at: now + ACCESS_TOKEN_TTL,
      refresh_expires_at: now + REFRESH_TOKEN_TTL,
      access_token_digest: digest(access_token),
      refresh_token_digest: digest(refresh_token)
    )

    [session, access_token, refresh_token]
  end

  def self.lookup_by_access_token(token)
    active.find_by(access_token_digest: digest(token))
  end

  def self.lookup_by_refresh_token(token)
    active.find_by(refresh_token_digest: digest(token))
  end

  def self.digest(token)
    Digest::SHA256.hexdigest(token.to_s)
  end

  def active_access_token?
    revoked_at.nil? && access_expires_at.future?
  end

  def active_refresh_token?
    revoked_at.nil? && refresh_expires_at.future?
  end

  def rotate_tokens!
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

  def revoke!
    update!(revoked_at: Time.current)
  end

  def touch_last_used!
    update!(last_used_at: Time.current)
  end

  private

  def self.build_token
    "mt_#{SecureRandom.urlsafe_base64(48)}"
  end
  private_class_method :build_token
end
