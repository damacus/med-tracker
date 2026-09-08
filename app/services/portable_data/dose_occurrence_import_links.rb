module PortableData
  class DoseOccurrenceImportLinks
    def initialize(household:, payload:)
      @household = household
      @payload = payload
    end

    def errors
      takes = take_rows
      Array(@payload.dig(:records, :dose_occurrences)).each_with_index.filter_map do |row, index|
        next if row[:medication_take_portable_id].blank?
        next if matches?(row, takes[row[:medication_take_portable_id]])

        "dose_occurrences[#{index}] take does not match its source and window"
      end
    end

    private

    def take_rows
      rows = Array(@payload.dig(:records, :medication_takes))
      serializer = ExportEventRecordSerializer.new
      existing_takes.map { |take| serializer.medication_take_payload(take).with_indifferent_access }
                    .index_by { |row| row[:portable_id] }.merge(rows.index_by { |row| row[:portable_id] })
    end

    def existing_takes
      ids = Array(@payload.dig(:records, :dose_occurrences)).pluck(:medication_take_portable_id)
      MedicationTake.where(household: @household, portable_id: ids)
                    .includes(:schedule, :person_medication, :taken_from_medication, :taken_from_location)
    end

    def matches?(row, take)
      take && take[:source_type] == row[:source_type] && take[:source_portable_id] == row[:source_portable_id] &&
        Time.iso8601(take[:taken_at]).in_time_zone.to_date.iso8601 == row[:window_starts_on]
    rescue ArgumentError, TypeError
      false
    end
  end
end
