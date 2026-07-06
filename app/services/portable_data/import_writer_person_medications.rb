# frozen_string_literal: true

module PortableData
  module ImportWriterPersonMedications
    def import_person_medications
      records(:person_medications).each do |row|
        person_medication = find_or_initialize(PersonMedication, row)
        person_medication.assign_attributes(person_medication_attributes(row))
        person_medication.save!
      end
    end

    def person_medication_attributes(row)
      person_medication_subject_attributes(row).merge(person_medication_dose_attributes(row))
                                               .merge(person_medication_state_attributes(row))
    end

    def person_medication_subject_attributes(row)
      {
        person: person_by_portable_id(row.fetch(:person_portable_id)),
        medication: medication_by_portable_id(row.fetch(:medication_portable_id)),
        source_dosage_option: dosage_by_portable_id(row[:source_dosage_option_portable_id])
      }
    end

    def person_medication_dose_attributes(row)
      {
        dose_amount: row[:dose_amount],
        dose_unit: row[:dose_unit],
        dose_cycle: row[:dose_cycle].presence || :daily,
        max_daily_doses: row[:max_daily_doses],
        min_hours_between_doses: row[:min_hours_between_doses]
      }
    end

    def person_medication_state_attributes(row)
      attributes = {
        administration_kind: row[:administration_kind].presence || :as_needed,
        active: row.fetch(:active, true),
        notes: row[:notes]
      }
      attributes[:position] = row[:position] if row.key?(:position) && !row[:position].nil?
      attributes
    end
  end
end
