# frozen_string_literal: true

module PortableData
  module ImportWriterRecords
    include ImportWriterMedicationTakes

    def import_notification_preferences
      records(:notification_preferences).each do |row|
        preference = find_or_initialize(NotificationPreference, row)
        preference.assign_attributes(notification_preference_attributes(row))
        preference.save!
      end
    end

    def notification_preference_attributes(row)
      notification_preference_flags(row).merge(notification_preference_times(row)).merge(
        person: person_by_portable_id(row.fetch(:person_portable_id))
      )
    end

    def notification_preference_flags(row)
      {
        enabled: row.fetch(:enabled, true),
        dose_due_enabled: row.fetch(:dose_due_enabled, true),
        missed_dose_enabled: row.fetch(:missed_dose_enabled, true),
        low_stock_enabled: row.fetch(:low_stock_enabled, true),
        private_text_enabled: row.fetch(:private_text_enabled, false)
      }
    end

    def notification_preference_times(row)
      {
        morning_time: row[:morning_time],
        afternoon_time: row[:afternoon_time],
        evening_time: row[:evening_time],
        night_time: row[:night_time]
      }
    end

    def restore_exported_inventory
      records(:medications).each do |row|
        medication_by_portable_id(row.fetch(:portable_id)).update!(
          current_supply: row[:current_supply],
          reorder_threshold: row[:reorder_threshold] || 0
        )
      end
    end

    def find_or_initialize(model, row)
      model.find_or_initialize_by(household: household, portable_id: row.fetch(:portable_id))
    end

    def location_by_portable_id(portable_id)
      return if portable_id.blank?

      Location.find_by!(household: household, portable_id: portable_id)
    end

    def person_by_portable_id(portable_id)
      Person.find_by!(household: household, portable_id: portable_id)
    end

    def medication_by_portable_id(portable_id)
      return if portable_id.blank?

      Medication.find_by!(household: household, portable_id: portable_id)
    end

    def dosage_by_portable_id(portable_id)
      return if portable_id.blank?

      MedicationDosageOption.find_by!(household: household, portable_id: portable_id)
    end

    def source_record(source_type, portable_id)
      case source_type
      when 'schedule' then Schedule.find_by!(household: household, portable_id: portable_id)
      when 'person_medication' then PersonMedication.find_by!(household: household, portable_id: portable_id)
      else raise Importer::Error, 'Unsupported medication take source type'
      end
    end
  end
end
