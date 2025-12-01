# frozen_string_literal: true

# Person captures an individual's demographic details and their medication plan.
class Person < ApplicationRecord
  # Audit trail for patient/carer demographic changes
  # Critical for compliance: tracks all changes to personal data
  # Tracks: name, DOB, email, person_type, has_capacity changes
  # @see docs/audit-trail.md
  has_paper_trail

  belongs_to :account, optional: true
  has_one :user, inverse_of: :person, dependent: :destroy
  has_many :prescriptions, dependent: :destroy
  has_many :medicines, through: :prescriptions
  has_many :person_medicines, dependent: :destroy
  has_many :non_prescription_medicines, through: :person_medicines, source: :medicine
  has_many :carer_relationships, foreign_key: :patient_id, dependent: :destroy, inverse_of: :patient
  has_many :carers, through: :carer_relationships, source: :carer

  has_many :patient_relationships, class_name: 'CarerRelationship',
                                   foreign_key: :carer_id,
                                   dependent: :destroy,
                                   inverse_of: :carer
  has_many :patients, through: :patient_relationships, source: :patient

  normalizes :email, with: ->(email) { email&.strip&.downcase }

  enum :person_type, {
    adult: 0,
    minor: 1,
    dependent_adult: 2
  }

  validates :date_of_birth, presence: true
  validates :name, presence: true
  validates :email, allow_blank: true,
                    format: { with: URI::MailTo::EMAIL_REGEXP, allow_blank: true },
                    uniqueness: { allow_blank: true }
  validate :carer_required_when_lacking_capacity

  scope :without_carers, -> { where.missing(:carer_relationships) }

  def age(reference_date = Time.zone.today)
    return nil unless date_of_birth

    years = reference_date.year - date_of_birth.year
    return years if birthday_passed?(reference_date)

    years - 1
  end

  def adult?
    age >= 18 || person_type == 'adult'
  end

  def minor?
    age < 18 && person_type == 'minor'
  end

  def dependent_adult?
    age >= 18 && person_type == 'dependent_adult'
  end

  def needs_carer?
    (minor? || dependent_adult?) && carers.empty?
  end

  private

  def birthday_passed?(reference_date)
    month_day(reference_date) >= month_day(date_of_birth)
  end

  def month_day(date)
    (date.month * 100) + date.day
  end

  def carer_required_when_lacking_capacity
    return if has_capacity
    return if carer_relationships.any?

    errors.add(:base, 'A person without capacity must have at least one carer assigned')
  end
end
