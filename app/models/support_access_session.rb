# frozen_string_literal: true

class SupportAccessSession < ApplicationRecord
  has_paper_trail

  belongs_to :platform_admin
  belongs_to :household

  validates :reason, :mfa_verified_at, :starts_at, :expires_at, presence: true
  validate :expires_after_start

  before_validation :set_time_window

  scope :active, lambda {
    where(ended_at: nil, expired_at: nil)
      .where(starts_at: ..Time.current)
      .where('expires_at > ?', Time.current)
  }

  def active?
    ended_at.nil? && expired_at.nil? && starts_at <= Time.current && expires_at > Time.current
  end

  private

  def set_time_window
    self.starts_at ||= Time.current
    self.expires_at ||= starts_at + 30.minutes if starts_at
  end

  def expires_after_start
    return if starts_at.blank? || expires_at.blank? || expires_at > starts_at

    errors.add(:expires_at, 'must be after starts at')
  end
end
