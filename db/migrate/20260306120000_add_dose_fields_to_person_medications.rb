class AddDoseFieldsToPersonMedications < ActiveRecord::Migration[8.0]
  def up
    add_column :person_medications, :dose_amount, :decimal, precision: 10, scale: 2
    add_column :person_medications, :dose_unit, :string

    PersonMedication.reset_column_information

    PersonMedication.includes(:person, medication: :dosages).find_each do |person_medication|
      next if person_medication.dose_amount.present? && person_medication.dose_unit.present?

      dosage = person_medication.medication.default_dosage_for_person_type(person_medication.person.person_type) ||
               person_medication.medication.dosages.first
      amount = dosage&.amount || person_medication.medication.dosage_amount
      unit = dosage&.unit || person_medication.medication.dosage_unit
      next if amount.blank? || unit.blank?

      person_medication.update_columns(dose_amount: amount, dose_unit: unit)
    end
  end

  def down
    remove_column :person_medications, :dose_amount
    remove_column :person_medications, :dose_unit
  end
end
