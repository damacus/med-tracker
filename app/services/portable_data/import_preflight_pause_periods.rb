module PortableData
  module ImportPreflightPausePeriods
    def validate_pause_period_references(errors)
      records(:medication_pause_periods).each_with_index do |row, index|
        target = { 'schedule' => 'schedules', 'person_medication' => 'person_medications' }[row[:source_type]]
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
