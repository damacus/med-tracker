# frozen_string_literal: true

FactoryBot.define do
  factory :schedule do
    person
    medication

    transient do
      dosage do
        medication.dosage_records.adult_default.first ||
          medication.dosage_records.order(:amount, :id).first ||
          association(:dosage, medication: medication)
      end
    end

    source_dosage_option do
      dosage if dosage.is_a?(MedicationDosageOption)
    end
    dose_amount { dosage.amount }
    dose_unit { dosage.unit }
    frequency { 'As needed' }
    start_date { Time.zone.today }
    end_date { 1.year.from_now.to_date }
    max_daily_doses { 4 }
    min_hours_between_doses { 4 }
    dose_cycle { :daily }
    schedule_type { :daily }
    schedule_config { {} }

    trait :weekly do
      dose_cycle { :weekly }
      max_daily_doses { 1 }
      frequency { 'Once weekly' }
    end

    trait :monthly do
      dose_cycle { :monthly }
      max_daily_doses { 1 }
      frequency { 'Once monthly' }
    end

    trait :expired do
      start_date { 1.year.ago.to_date }
      end_date { 1.day.ago.to_date }
    end
  end
end
