# frozen_string_literal: true

# Person captures an individual's demographic details and their medication plan.
class Person < ApplicationRecord
  include Auditable

  has_one :user, inverse_of: :person, dependent: :destroy
  has_many :prescriptions, dependent: :destroy
  has_many :medicines, through: :prescriptions
  has_many :person_medicines, dependent: :destroy
  has_many :non_prescription_medicines, through: :person_medicines, source: :medicine

  # Carer relationships - people who care for this person
  has_many :carer_relationships, foreign_key: :patient_id, dependent: :destroy, inverse_of: :patient
  has_many :carers, through: :carer_relationships, source: :carer

  # Patient relationships - people this person cares for
  has_many :patient_relationships, class_name: 'CarerRelationship',
                                   foreign_key: :carer_id,
                                   dependent: :destroy,
                                   inverse_of: :carer
  has_many :patients, through: :patient_relationships, source: :patient

  normalizes :email, with: ->(email) { email&.strip&.downcase }

  enum :person_type, {
    adult: 0,            # Self-managing adult
    minor: 1,            # Child requiring parental consent
    dependent_adult: 2   # Adult requiring carer support
  }

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

  def adult?(age_threshold = 18)
    return false unless age

    age >= age_threshold
  end

  def minor?(age_threshold = 18)
    !adult?(age_threshold)
  end

  private

  def birthday_passed?(reference_date)
    month_day(reference_date) >= month_day(date_of_birth)
  end

  def month_day(date)
    (date.month * 100) + date.day
  end
end
