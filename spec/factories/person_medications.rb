# frozen_string_literal: true

FactoryBot.define do
  factory :person_medication do
    person
    medication

    transient do
      dosage do
        if %w[minor dependent_adult].include?(person.person_type.to_s)
          medication.dosage_records.child_default.first ||
            medication.dosage_records.adult_default.first ||
            medication.dosage_records.order(:amount, :id).first
        else
          medication.dosage_records.adult_default.first ||
            medication.dosage_records.order(:amount, :id).first
        end
      end
    end

    source_dosage_option do
      dosage if dosage.is_a?(MedicationDosageOption)
    end
    dose_amount { dosage&.amount || medication.dosage_amount }
    dose_unit { dosage&.unit || medication.dosage_unit }
    notes { nil }
    max_daily_doses { nil }
    min_hours_between_doses { nil }

    trait :with_max_doses do
      max_daily_doses { 3 }
    end

    trait :with_min_hours do
      min_hours_between_doses { 4 }
    end

    trait :with_both_restrictions do
      max_daily_doses { 2 }
      min_hours_between_doses { 12 }
    end
  end
end
