# frozen_string_literal: true

# Person captures an individual's demographic details and their medication plan.
class Person < ApplicationRecord
  has_one :user, inverse_of: :person, dependent: :destroy
  has_many :prescriptions, dependent: :destroy
  has_many :medicines, through: :prescriptions
  normalizes :email, with: ->(email) { email&.strip&.downcase }

  validates :date_of_birth, presence: true
  validates :name, presence: true
  validates :email, allow_blank: true,
                    format: { with: URI::MailTo::EMAIL_REGEXP, allow_blank: true },
                    uniqueness: { allow_blank: true }

  def age(reference_date = Time.zone.today)
    return nil unless date_of_birth

    years = reference_date.year - date_of_birth.year
    return years if birthday_passed?(reference_date)

    years - 1
  end

  private

  def birthday_passed?(reference_date)
    month_day(reference_date) >= month_day(date_of_birth)
  end

  def month_day(date)
    (date.month * 100) + date.day
  end
end
