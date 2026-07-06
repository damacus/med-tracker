# frozen_string_literal: true

module PortableData
  class ImportPreflight
    Result = Data.define(:conflicts, :errors) do
      def blocked? = conflicts.any? || errors.any?
    end

    def initialize(household:, payload:)
      @household = household
      @payload = payload
    end

    def call
      Result.new(conflicts: conflict_reports, errors: preflight_errors)
    end

    private

    attr_reader :household, :payload

    def preflight_errors
      rails_id_errors + person_capacity_errors + missing_reference_errors
    end

    def rails_id_errors
      payload.fetch(:records).flat_map do |record_type, rows|
        Array(rows).each_with_index.filter_map do |row, index|
          forbidden_keys = row.to_h.keys.map(&:to_s).select { |key| rails_id_key?(key) }
          next if forbidden_keys.empty?

          "#{record_type}[#{index}] includes Rails numeric IDs: #{forbidden_keys.sort.join(', ')}"
        end
      end
    end

    def rails_id_key?(key)
      key == 'id' || (key.end_with?('_id') && key.exclude?('portable'))
    end

    def person_capacity_errors
      records(:people).each_with_index.filter_map do |row, index|
        next unless row[:person_type].to_s.in?(%w[minor dependent_adult])
        next unless row.key?(:has_capacity) && ActiveModel::Type::Boolean.new.cast(row[:has_capacity])

        "people[#{index}].has_capacity must be false for minors and dependent adults"
      end
    end

    def missing_reference_errors
      errors = []
      validate_people_references(errors)
      validate_medication_references(errors)
      validate_dosage_references(errors)
      validate_schedule_references(errors)
      validate_person_medication_references(errors)
      validate_medication_take_references(errors)
      validate_notification_preference_references(errors)
      errors
    end

    def validate_people_references(errors)
      records(:people).each_with_index do |row, index|
        Array(row[:location_portable_ids]).each do |portable_id|
          add_missing_reference_error(errors, reference('people', index, 'location_portable_ids', 'locations',
                                                        portable_id))
        end
      end
    end

    def validate_medication_references(errors)
      records(:medications).each_with_index do |row, index|
        add_missing_reference_error(errors, optional_reference('medications', index, 'location_portable_id',
                                                               'locations', row[:location_portable_id]))
      end
    end

    def validate_dosage_references(errors)
      records(:dosage_options).each_with_index do |row, index|
        add_missing_reference_error(errors, reference('dosage_options', index, 'medication_portable_id',
                                                      'medications', row[:medication_portable_id]))
      end
    end

    def validate_schedule_references(errors)
      records(:schedules).each_with_index do |row, index|
        schedule_references(row, index).each { |reference| add_missing_reference_error(errors, reference) }
      end
    end

    def schedule_references(row, index)
      [
        reference('schedules', index, 'person_portable_id', 'people', row[:person_portable_id]),
        reference('schedules', index, 'medication_portable_id', 'medications', row[:medication_portable_id]),
        optional_reference('schedules', index, 'source_dosage_option_portable_id', 'dosage_options',
                           row[:source_dosage_option_portable_id])
      ]
    end

    def validate_person_medication_references(errors)
      records(:person_medications).each_with_index do |row, index|
        person_medication_references(row, index).each { |reference| add_missing_reference_error(errors, reference) }
      end
    end

    def person_medication_references(row, index)
      [
        reference('person_medications', index, 'person_portable_id', 'people', row[:person_portable_id]),
        reference('person_medications', index, 'medication_portable_id', 'medications', row[:medication_portable_id]),
        optional_reference('person_medications', index, 'source_dosage_option_portable_id', 'dosage_options',
                           row[:source_dosage_option_portable_id])
      ]
    end

    def validate_medication_take_references(errors)
      records(:medication_takes).each_with_index do |row, index|
        medication_take_references(row, index).each { |reference| add_missing_reference_error(errors, reference) }
      end
    end

    def medication_take_references(row, index)
      source_record_type = row[:source_type] == 'person_medication' ? 'person_medications' : 'schedules'

      [
        reference('medication_takes', index, 'source_portable_id', source_record_type, row[:source_portable_id]),
        optional_reference('medication_takes', index, 'taken_from_medication_portable_id', 'medications',
                           row[:taken_from_medication_portable_id]),
        optional_reference('medication_takes', index, 'taken_from_location_portable_id', 'locations',
                           row[:taken_from_location_portable_id])
      ]
    end

    def validate_notification_preference_references(errors)
      records(:notification_preferences).each_with_index do |row, index|
        add_missing_reference_error(errors, reference('notification_preferences', index, 'person_portable_id',
                                                      'people', row[:person_portable_id]))
      end
    end

    def reference(record_type, index, field, target_type, portable_id)
      {
        record_type: record_type,
        index: index,
        field: field,
        target_type: target_type,
        portable_id: portable_id,
        required: true
      }
    end

    def optional_reference(record_type, index, field, target_type, portable_id)
      reference(record_type, index, field, target_type, portable_id).merge(required: false)
    end

    def add_missing_reference_error(errors, reference)
      portable_id = reference.fetch(:portable_id)
      return if portable_id.blank? && !reference.fetch(:required)
      return if portable_ids_for(reference.fetch(:target_type)).include?(portable_id)

      errors << missing_reference_message(reference)
    end

    def missing_reference_message(reference)
      record_type = reference.fetch(:record_type)
      index = reference.fetch(:index)
      field = reference.fetch(:field)
      target_type = reference.fetch(:target_type)

      "#{record_type}[#{index}].#{field} references unknown #{target_type} portable ID"
    end

    def portable_ids_for(record_type)
      @portable_ids_for ||= {}
      @portable_ids_for[record_type] ||= begin
        ids = records(record_type).pluck(:portable_id)
        ids.concat(model_for_record_type(record_type).where(household: household).pluck(:portable_id))
        ids.compact.uniq
      end
    end

    def model_for_record_type(record_type)
      {
        'people' => Person,
        'locations' => Location,
        'medications' => Medication,
        'dosage_options' => MedicationDosageOption,
        'schedules' => Schedule,
        'person_medications' => PersonMedication,
        'medication_takes' => MedicationTake,
        'notification_preferences' => NotificationPreference
      }.fetch(record_type.to_s)
    end

    def conflict_reports
      location_conflicts + person_conflicts + medication_conflicts
    end

    def location_conflicts
      records(:locations).filter_map do |row|
        existing = household.locations
                            .where('LOWER(name) = ?', row[:name].to_s.downcase)
                            .where.not(portable_id: row[:portable_id])
                            .first
        conflict_payload('locations', row, existing, 'name') if existing
      end
    end

    def person_conflicts
      records(:people).filter_map do |row|
        next if row[:email].blank?

        existing = household.people
                            .where(email: row[:email])
                            .where.not(portable_id: row[:portable_id])
                            .first
        conflict_payload('people', row, existing, 'email') if existing
      end
    end

    def medication_conflicts
      records(:medications).filter_map do |row|
        existing = household.medications
                            .where('LOWER(name) = ?', row[:name].to_s.downcase)
                            .where.not(portable_id: row[:portable_id])
                            .first
        conflict_payload('medications', row, existing, 'name') if existing
      end
    end

    def conflict_payload(record_type, row, existing, field)
      {
        record_type: record_type,
        portable_id: row[:portable_id],
        field: field,
        existing_portable_id: existing.portable_id
      }
    end

    def records(name)
      Array(payload.dig(:records, name)).map(&:with_indifferent_access)
    end
  end
end
