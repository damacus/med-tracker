# frozen_string_literal: true

require 'digest'

class HouseholdInvitation < ApplicationRecord
  attr_reader :plain_token
  attr_accessor :dependent_ids, :access_level, :relationship_type

  has_paper_trail ignore: %i[token_digest]

  belongs_to :household
  belongs_to :invited_by_membership, class_name: 'HouseholdMembership'

  has_many :household_invitation_grants, dependent: :destroy

  enum :membership_role, { administrator: 'administrator', member: 'member' }, validate: true

  before_validation :assign_token_digest, on: :create
  before_validation :set_expires_at

  normalizes :email, with: ->(email) { email.to_s.strip.downcase.presence }

  validates :email, :token_digest, :expires_at, presence: true
  validates :email,
            format: { with: URI::MailTo::EMAIL_REGEXP },
            uniqueness: {
              scope: :household_id,
              case_sensitive: false,
              conditions: -> { where(accepted_at: nil, revoked_at: nil) }
            }
  validate :inviter_must_belong_to_household

  scope :pending, -> { where(accepted_at: nil, revoked_at: nil).where('expires_at > ?', Time.current) }
  scope :expired, -> { where(accepted_at: nil, revoked_at: nil).where(expires_at: ..Time.current) }

  def expired?
    return false if expires_at.nil?

    accepted_at.nil? && revoked_at.nil? && expires_at < Time.current
  end

  def accepted?
    accepted_at.present?
  end

  def revoked?
    revoked_at.present?
  end

  def pending?
    !accepted? && !revoked? && !expired?
  end

  def resendable?
    return false if accepted? || revoked?

    !self.class.pending.where(household_id: household_id, email: email).where.not(id: id).exists?
  end

  def cancellable?
    !accepted? && !revoked?
  end

  def resend!
    raise ActiveRecord::RecordInvalid, self unless resendable?

    self.paper_trail_event = 'resend'
    assign_token_digest
    update!(token_digest: token_digest, expires_at: 7.days.from_now)
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

  def inviter_must_belong_to_household
    return if invited_by_membership.blank? || invited_by_membership.household_id == household_id

    errors.add(:invited_by_membership, 'must belong to the same household')
  end
end
