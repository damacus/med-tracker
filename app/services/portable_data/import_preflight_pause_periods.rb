# frozen_string_literal: true

module PortableData
  module ImportPreflightPausePeriods
    def validate_pause_period_references(errors)
      targets_by_type = { 'schedule' => 'schedules', 'person_medication' => 'person_medications' }
      records(:medication_pause_periods).each_with_index do |row, index|
        target = targets_by_type[row[:source_type]]
        if target
          add_missing_reference_error(errors, reference('medication_pause_periods', index,
                                                        'source_portable_id', target, row[:source_portable_id]))
        else
          errors << "medication_pause_periods[#{index}].source_type is unsupported"
        end
      end
    end
  end
end
