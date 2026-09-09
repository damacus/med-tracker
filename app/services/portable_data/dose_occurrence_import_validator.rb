module PortableData
  class DoseOccurrenceImportValidator
    def initialize(row)
      @row = row
    end

    def valid?
      valid_identity? && valid_times? && valid_context? && valid_resolution?
    rescue ArgumentError, TypeError
      false
    end

    private

    def valid_identity?
      valid_types? && valid_position? && valid_date? && valid_window?
    end

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
      return false if @row[:source_type] == 'person_medication' && @row[:scheduled_at].present?

      %i[scheduled_at resolved_at].all? { |field| @row[field].nil? || Time.iso8601(@row[field]).present? }
    end

    def valid_window?
      return true unless @row.key?(:window_ends_on)

      first = Date.iso8601(@row[:window_starts_on])
      last = Date.iso8601(@row[:window_ends_on])
      return false unless last.iso8601 == @row[:window_ends_on]
      return last == first if @row[:source_type] == 'schedule'

      valid_routine_window?(first, last)
    end

    def valid_routine_window?(first, last)
      last == first || (first.monday? && last == first + 6) ||
        (first == first.beginning_of_month && last == first.end_of_month)
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
