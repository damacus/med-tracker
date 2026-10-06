module PortableData
  class DoseOccurrenceImportLinks
    def initialize(household:, payload:)
      @household = household
      @payload = payload
    end

    def errors
      takes = take_rows
      Array(@payload.dig(:records, :dose_occurrences)).each_with_index.filter_map do |row, index|
        next "dose_occurrences[#{index}] source is not a routine assignment" unless source_allowed?(row)
        next if row[:medication_take_portable_id].blank?
        next if matches?(row, takes[row[:medication_take_portable_id]])

        "dose_occurrences[#{index}] take does not match its source and window"
      end
    end

    private

    def source_allowed?(row)
      return true unless row[:source_type] == 'person_medication'

      source = assignments[row[:source_portable_id]]
      source && source[:administration_kind] == 'routine' && PersonMedication.dose_cycles.key?(source[:dose_cycle])
    end

    def assignments
      @assignments ||= begin
        rows = Array(@payload.dig(:records, :person_medications))
        ids = Array(@payload.dig(:records, :dose_occurrences)).pluck(:source_portable_id)
        existing = PersonMedication.where(household: @household, portable_id: ids).to_h do |source|
          [source.portable_id, { administration_kind: source.administration_kind, dose_cycle: source.dose_cycle }]
        end
        existing.merge(rows.index_by { |row| row[:portable_id] })
      end
    end

    def take_rows
      rows = Array(@payload.dig(:records, :medication_takes))
      serializer = ExportEventRecordSerializer.new
      existing = existing_takes.map { |take| serializer.medication_take_payload(take).with_indifferent_access }
                               .index_by { |row| row[:portable_id] }
      rows.index_by { |row| row[:portable_id] }.merge(existing)
    end

    def existing_takes
      ids = Array(@payload.dig(:records, :dose_occurrences)).pluck(:medication_take_portable_id)
      MedicationTake.where(household: @household, portable_id: ids)
                    .includes(:schedule, :person_medication, :taken_from_medication, :taken_from_location)
    end

    def matches?(row, take)
      take && take[:source_type] == row[:source_type] && take[:source_portable_id] == row[:source_portable_id] &&
        window(row).cover?(Time.iso8601(take[:taken_at]).in_time_zone.to_date)
    rescue ArgumentError, TypeError
      false
    end

    def window(row)
      first = Date.iso8601(row[:window_starts_on])
      last = row[:window_ends_on] ? Date.iso8601(row[:window_ends_on]) : legacy_window_end(row, first)
      first..last
    end

    def legacy_window_end(row, first)
      return first if row[:source_type] == 'schedule'

      cycle = assignments.fetch(row[:source_portable_id]).fetch(:dose_cycle)
      DoseCycle.new(cycle).range_for(first.in_time_zone).end.to_date
    end
  end
end
