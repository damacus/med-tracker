# frozen_string_literal: true

FactoryBot.define do
  factory :dosage, class: 'MedicationDosageOption' do
    medication
    amount { 500 }
    unit { 'mg' }
    frequency { 'As needed' }
    description { 'Standard dose' }
    default_max_daily_doses { 1 }
    default_min_hours_between_doses { 24 }
    default_dose_cycle { :daily }
  end
end
