class Medicine < ApplicationRecord
  has_many :prescriptions, dependent: :destroy
  has_many :people, through: :prescriptions
  has_many :recommended_dosages, dependent: :destroy

  validates :name, presence: true, uniqueness: true
  validates :description, presence: true
  validates :standard_dosage, presence: true

  def find_recommended_dosage(age)
    recommended_dosages.find_for_age(age)
  end

  # Get all recommended dosages ordered by age range
  def dosage_instructions
    recommended_dosages.order(:min_age).map do |dosage|
      "#{dosage.description}\n#{dosage.dosage_instruction}"
    end.join("\n")
  end
end
