class Person < ApplicationRecord
  has_many :prescriptions, dependent: :destroy
  has_many :medicines, through: :prescriptions

  validates :name, presence: true
  validates :date_of_birth, presence: true

  def age
    return nil unless date_of_birth

    now = Time.current.to_date
    age = now.year - date_of_birth.year
    age -= 1 if now.month < date_of_birth.month ||
               (now.month == date_of_birth.month && now.day < date_of_birth.day)
    age
  end
end
