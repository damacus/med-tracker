class Medicine < ApplicationRecord
  has_many :prescriptions, dependent: :destroy
  has_many :take_medicines, through: :prescriptions, dependent: :destroy
  has_many :people, through: :prescriptions
  has_many :dosage_options, dependent: :destroy
  accepts_nested_attributes_for :dosage_options, allow_destroy: true, reject_if: :all_blank

  validates :name, presence: true, uniqueness: true
  validates :description, presence: true
  validates :dosage, presence: true
  validates :unit, presence: true

  def available_dosages
    dosage_options.ordered
  end

  def dosage_instructions
    "#{dosage} #{unit}"
  end
end
