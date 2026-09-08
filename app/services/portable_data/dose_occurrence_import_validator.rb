module PortableData
  class DoseOccurrenceImportValidator
    def initialize(row)
      @row = row
    end

    def valid?
      valid_types? &&
        valid_position? && valid_date? && valid_times? && valid_context? && valid_resolution?
    rescue ArgumentError, TypeError
      false
    end

    private

    def valid_types?
      %w[schedule person_medication].include?(@row[:source_type]) &&
        MedicationDoseOccurrence::OUTCOMES.include?(@row[:outcome])
    end

    def valid_position?
      @row[:position].is_a?(Integer) && @row[:position].between?(1, 2_147_483_647)
    end

    def valid_date?
      Date.iso8601(@row[:window_starts_on]).iso8601 == @row[:window_starts_on]
    end

    def valid_times?
      %i[scheduled_at resolved_at].all? { |field| @row[field].nil? || Time.iso8601(@row[field]).present? }
    end

    def valid_context?
      reason = @row[:reason]
      return false unless valid_note?
      return false unless reason.nil? || MedicationDoseOccurrence::REASONS.include?(reason)

      @row[:outcome] == 'not_taken' || (reason.blank? && @row[:note].blank?)
    end

    def valid_note?
      note = @row[:note]
      note.nil? || (note.is_a?(String) && note.length <= 2000)
    end

    def valid_resolution?
      @row[:medication_take_portable_id].present? == (@row[:outcome] == 'taken') &&
        @row[:resolved_at].present? == (@row[:outcome] != 'open')
    end
  end
end
