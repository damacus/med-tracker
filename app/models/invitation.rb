# frozen_string_literal: true

class Invitation < ApplicationRecord
  has_paper_trail

  before_create :generate_token
  before_create :set_expires_at

  validates :email, presence: true,
                    format: { with: URI::MailTo::EMAIL_REGEXP },
                    uniqueness: { case_sensitive: false, conditions: -> { pending } }
  validates :role, presence: true
  validate :role_cannot_be_minor

  enum :role, { administrator: 0, doctor: 1, nurse: 2, carer: 3, parent: 4, minor: 5 }, validate: true

  scope :pending, -> { where(accepted_at: nil).where('expires_at > ?', Time.current) }

  def self.assignable_roles
    roles.except('minor')
  end

  def expired?
    return false if expires_at.nil?

    expires_at < Time.current
  end

  def accepted?
    accepted_at.present?
  end

  def pending?
    !accepted? && !expired?
  end

  def resendable?
    return false if accepted?

    !self.class.pending.where.not(id: id).exists?(email: email)
  end

  def resend!
    raise ActiveRecord::RecordInvalid, self unless resendable?

    self.paper_trail_event = 'resend'
    update!(token: SecureRandom.hex(32), expires_at: 7.days.from_now)
  end

  private

  def generate_token
    self.token = SecureRandom.hex(32)
  end

  def set_expires_at
    self.expires_at ||= 7.days.from_now
  end

  def role_cannot_be_minor
    errors.add(:role, 'Children must be added by a parent or carer.') if minor?
  end
end
