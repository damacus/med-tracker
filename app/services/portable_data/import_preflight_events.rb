module PortableData
  module ImportPreflightEvents
    private

    def preflight_errors
      rails_id_errors + person_capacity_errors + missing_reference_errors +
        DoseOccurrenceImportPreflight.new(household: household, rows: records(:dose_occurrences)).errors +
        DoseOccurrenceImportLinks.new(household: household, payload: payload).errors
    end

    def validate_dose_occurrence_references(errors)
      records(:dose_occurrences).each_with_index do |row, index|
        source_type = row[:source_type] == 'person_medication' ? 'person_medications' : 'schedules'
        add_missing_reference_error(errors, reference('dose_occurrences', index, 'source_portable_id',
                                                      source_type, row[:source_portable_id]))
        add_missing_reference_error(errors, optional_reference('dose_occurrences', index, 'medication_take_portable_id',
                                                               'medication_takes', row[:medication_take_portable_id]))
      end
    end

    def validate_health_event_references(errors)
      records(:health_events).each_with_index do |row, index|
        add_missing_reference_error(errors, reference('health_events', index, 'person_portable_id',
                                                      'people', row[:person_portable_id]))
        Array(row[:medication_portable_ids]).each do |portable_id|
          add_missing_reference_error(errors, reference('health_events', index, 'medication_portable_ids',
                                                        'medications', portable_id))
        end
      end
    end
  end
end
