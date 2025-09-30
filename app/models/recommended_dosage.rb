# frozen_string_literal: true

class RecommendedDosage < ApplicationRecord
  belongs_to :medicine

  validates :min_age, presence: true, numericality: { greater_than_or_equal_to: 0 }
  validates :max_age, presence: true, numericality: { greater_than: :min_age }
  validates :amount_ml, presence: true, numericality: { greater_than: 0 }
  validates :frequency_per_day, presence: true, numericality: { greater_than: 0 }

  def description
    if max_age < 16
      "Children #{min_age}-#{max_age} years"
    else
      "Adults and children over #{min_age} years"
    end
  end

  def dosage_instruction
    "#{amount_ml}ml up to #{frequency_per_day} times in 24 hours"
  end

  # Find recommended dosage for a person based on their age
  def self.find_for_age(age)
    where('min_age <= ? AND max_age >= ?', age, age).first
  end
end
