# frozen_string_literal: true

class Invitation < ApplicationRecord
  before_create :generate_token
  before_create :set_expires_at

  validates :email, presence: true,
                    format: { with: URI::MailTo::EMAIL_REGEXP },
                    uniqueness: { case_sensitive: false, conditions: -> { pending } }
  validates :role, presence: true

  enum :role, { administrator: 0, doctor: 1, nurse: 2, carer: 3, parent: 4, minor: 5 }, validate: true

  scope :pending, -> { where(accepted_at: nil).where('expires_at > ?', Time.current) }

  def expired?
    return false if expires_at.nil?

    expires_at < Time.current
  end

  def accepted?
    accepted_at.present?
  end

  private

  def generate_token
    self.token = SecureRandom.hex(32)
  end

  def set_expires_at
    self.expires_at ||= 7.days.from_now
  end
end
