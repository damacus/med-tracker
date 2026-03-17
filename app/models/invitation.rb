# frozen_string_literal: true

require 'digest'

class Invitation < ApplicationRecord
  attr_reader :plain_token

  has_paper_trail ignore: %i[token_digest]

  before_create :assign_token_digest
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
    assign_token_digest
    update!(token_digest:, expires_at: 7.days.from_now)
  end

  def token
    plain_token
  end

  def self.digest(raw_token)
    return if raw_token.blank?

    Digest::SHA256.hexdigest(raw_token)
  end

  private

  def assign_token_digest
    @plain_token = SecureRandom.hex(32)
    self.token_digest = self.class.digest(@plain_token)
  end

  def set_expires_at
    self.expires_at ||= 7.days.from_now
  end

  def role_cannot_be_minor
    errors.add(:role, 'Children must be added by a parent or carer.') if minor?
  end
end
