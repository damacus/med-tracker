# frozen_string_literal: true

# User model for storing user information and authentication
class User < ApplicationRecord
  has_secure_password
  has_many :sessions, dependent: :destroy
  has_many :prescriptions, dependent: :destroy
  has_many :medicines, through: :prescriptions

  enum :role, { admin: 0, carer: 1, child: 2 }

  normalizes :email_address, with: ->(e) { e.strip.downcase }

  validates :name, presence: true
  validates :email_address, presence: true,
                            uniqueness: { case_sensitive: false },
                            format: { with: /\A[\w+\-.]+@[a-z\d\-.]+\.[a-z]+\z/i }
  validates :date_of_birth, presence: true
  validate :date_of_birth_cannot_be_in_the_future

  # Calculate user's age based on date of birth
  def age
    return nil unless date_of_birth

    now = Time.current.to_date
    age = now.year - date_of_birth.year
    age -= 1 if birthday_not_yet_occurred_this_year?(now)
    age
  end

  private

  # Determines if the birthday has not yet occurred in the current year
  def birthday_not_yet_occurred_this_year?(current_date)
    current_date.month < date_of_birth.month ||
      (current_date.month == date_of_birth.month && current_date.day < date_of_birth.day)
  end

  # Validates that date of birth is not in the future
  def date_of_birth_cannot_be_in_the_future
    return unless date_of_birth.present? && date_of_birth > Date.today

    errors.add(:date_of_birth, "can't be in the future")
  end
end
