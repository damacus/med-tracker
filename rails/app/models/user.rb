# frozen_string_literal: true

# User model for storing user information and authentication
class User < ApplicationRecord
  attr_accessor :dependent_ids, :membership_role, :dependent_access_level, :dependent_relationship_type

  # Audit trail for user account changes
  # Excludes password fields for security - passwords are never logged
  # Tracks: email changes, role changes, person associations
  # @see docs/audit-trail.md
  has_paper_trail ignore: %i[password_digest recovery_password_digest]

  belongs_to :person, inverse_of: :user

  accepts_nested_attributes_for :person

  has_secure_password validations: false
  has_many :schedules, through: :person
  has_many :medications, through: :schedules

  normalizes :email_address, with: ->(e) { e.strip.downcase }

  validates :email_address, presence: true,
                            uniqueness: { case_sensitive: false },
                            format: { with: /\A[\w+\-.]+@[a-z\d\-.]+\.[a-z]+\z/i }
  validates :password, length: { minimum: 8 }, allow_nil: true, if: -> { password.present? }

  scope :active, -> { where(active: true) }
  scope :inactive, -> { where(active: false) }

  def soft_deleted?
    account = person&.account
    person.present? && (account.nil? || account.closed?)
  end

  delegate :name, :date_of_birth, :age, to: :person, allow_nil: true

  def deactivate!
    transaction do
      update!(active: false)
      revoke_api_credentials!
    end
  end

  def activate!
    update!(active: true)
  end

  private

  def revoke_api_credentials!
    account = person&.account
    return unless account

    account.api_sessions.active.find_each(&:revoke!)
    account.api_app_tokens.active.find_each(&:revoke!)
  end
end
