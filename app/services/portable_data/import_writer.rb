# frozen_string_literal: true

module PortableData
  class ImportWriter
    include ImportWriterPersonMedications
    include ImportWriterRecords

    def initialize(household:, payload:)
      @household = household
      @payload = payload
    end

    def call
      import_locations
      import_people
      import_location_memberships
      import_medications
      import_dosage_options
      import_schedules
      import_person_medications
      import_medication_takes
      import_notification_preferences
      restore_exported_inventory
    end

    private

    attr_reader :household, :payload

    def records(name)
      Array(payload.dig(:records, name)).map(&:with_indifferent_access)
    end

    def import_locations
      records(:locations).each do |row|
        location = find_or_initialize(Location, row)
        location.assign_attributes(name: row[:name], description: row[:description])
        location.save!
      end
    end

    def import_people
      records(:people).each do |row|
        person = find_or_initialize(Person, row)
        person.primary_location = location_by_portable_id(Array(row[:location_portable_ids]).first)
        person.assign_attributes(person_attributes(row))
        person.save!
      end
    end

    def person_attributes(row)
      {
        name: row[:name],
        email: row[:email],
        date_of_birth: row[:date_of_birth],
        person_type: row[:person_type].presence || :adult,
        has_capacity: row.fetch(:has_capacity, true)
      }
    end

    def import_location_memberships
      records(:people).each do |row|
        person = person_by_portable_id(row.fetch(:portable_id))
        Array(row[:location_portable_ids]).each do |portable_id|
          location = location_by_portable_id(portable_id)
          next unless location

          LocationMembership.find_or_create_by!(household: household, person: person, location: location)
        end
      end
    end

    def import_medications
      records(:medications).each do |row|
        medication = find_or_initialize(Medication, row)
        medication.assign_attributes(medication_attributes(row))
        medication.save!
      end
    end

    def medication_attributes(row)
      attrs = medication_base_attributes(row).merge(medication_inventory_attributes(row))
                                             .merge(medication_catalog_attributes(row))
      dosage_options_for?(row) ? attrs : attrs.merge(medication_dose_attributes(row))
    end

    def medication_base_attributes(row)
      {
        location: location_by_portable_id(row[:location_portable_id]),
        name: row[:name],
        friendly_name: row[:friendly_name],
        category: row[:category],
        description: row[:description],
        default_schedule_type: row[:default_schedule_type].presence || :multiple_daily
      }
    end

    def medication_inventory_attributes(row)
      {
        current_supply: row[:current_supply],
        reorder_threshold: row[:reorder_threshold] || 0
      }
    end

    def medication_catalog_attributes(row)
      {
        barcode: row[:barcode],
        dmd_code: row[:dmd_code],
        dmd_system: row[:dmd_system],
        dmd_concept_class: row[:dmd_concept_class]
      }
    end

    def medication_dose_attributes(row)
      {
        dose_amount: row[:dose_amount],
        dose_unit: row[:dose_unit]
      }
    end

    def dosage_options_for?(medication_row)
      records(:dosage_options).any? do |row|
        row[:medication_portable_id] == medication_row.fetch(:portable_id)
      end
    end

    def import_dosage_options
      records(:dosage_options).each do |row|
        dosage = find_or_initialize(MedicationDosageOption, row)
        dosage.assign_attributes(dosage_attributes(row))
        dosage.save!
      end
    end

    def dosage_attributes(row)
      dosage_base_attributes(row).merge(dosage_default_attributes(row)).merge(dosage_inventory_attributes(row))
    end

    def dosage_base_attributes(row)
      {
        medication: medication_by_portable_id(row.fetch(:medication_portable_id)),
        amount: row[:amount],
        unit: row[:unit],
        frequency: row[:frequency],
        description: row[:description]
      }
    end

    def dosage_default_attributes(row)
      {
        default_for_adults: row[:default_for_adults] || false,
        default_for_children: row[:default_for_children] || false,
        default_max_daily_doses: row[:default_max_daily_doses],
        default_min_hours_between_doses: row[:default_min_hours_between_doses],
        default_dose_cycle: row[:default_dose_cycle].presence || :daily
      }
    end

    def dosage_inventory_attributes(row)
      {
        current_supply: row[:current_supply],
        reorder_threshold: row[:reorder_threshold]
      }
    end

    def import_schedules
      records(:schedules).each do |row|
        schedule = find_or_initialize(Schedule, row)
        schedule.assign_attributes(schedule_attributes(row))
        schedule.save!
      end
    end

    def schedule_attributes(row)
      schedule_subject_attributes(row).merge(schedule_dose_attributes(row)).merge(schedule_timing_attributes(row))
    end

    def schedule_subject_attributes(row)
      {
        person: person_by_portable_id(row.fetch(:person_portable_id)),
        medication: medication_by_portable_id(row.fetch(:medication_portable_id)),
        source_dosage_option: dosage_by_portable_id(row[:source_dosage_option_portable_id])
      }
    end

    def schedule_dose_attributes(row)
      {
        dose_amount: row[:dose_amount],
        dose_unit: row[:dose_unit],
        frequency: row[:frequency],
        dose_cycle: row[:dose_cycle].presence || :daily,
        max_daily_doses: row[:max_daily_doses],
        min_hours_between_doses: row[:min_hours_between_doses]
      }
    end

    def schedule_timing_attributes(row)
      {
        schedule_type: row[:schedule_type].presence || :daily,
        schedule_config: row[:schedule_config] || {},
        start_date: row[:start_date],
        end_date: row[:end_date],
        active: row.fetch(:active, true),
        notes: row[:notes]
      }
    end
  end
end
