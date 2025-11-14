# frozen_string_literal: true

# User model for storing user information and authentication
class User < ApplicationRecord
  # Audit trail for user account changes
  # Excludes password fields for security - passwords are never logged
  # Tracks: email changes, role changes, person associations
  # @see docs/audit-trail.md
  has_paper_trail ignore: %i[password_digest recovery_password_digest]

  belongs_to :person, inverse_of: :user

  accepts_nested_attributes_for :person

  has_secure_password validations: false
  has_many :sessions, dependent: :destroy
  has_many :prescriptions, through: :person
  has_many :medicines, through: :prescriptions

  normalizes :email_address, with: ->(e) { e.strip.downcase }

  validates :email_address, presence: true,
                            uniqueness: { case_sensitive: false },
                            format: { with: /\A[\w+\-.]+@[a-z\d\-.]+\.[a-z]+\z/i }
  validates :password, length: { minimum: 8 }, allow_nil: true, if: -> { password.present? }

  enum :role, { administrator: 0, doctor: 1, nurse: 2, carer: 3, parent: 4, minor: 5 }, validate: true

  delegate :name, :date_of_birth, :age, to: :person, allow_nil: true
end
