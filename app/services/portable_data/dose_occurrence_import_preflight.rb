module PortableData
  class DoseOccurrenceImportPreflight
    FIELDS = %w[source_type source_portable_id window_starts_on position scheduled_at outcome reason note resolved_at
                medication_take_portable_id].freeze

    def initialize(household:, rows:)
      @household = household
      @rows = rows
    end

    def errors
      @existing = existing_records
      @identities = @existing.values.index_by { |record| identity(DoseOccurrenceSerializer.new(record).as_json) }
      @take_links = existing_take_links
      @seen = Set.new
      @rows.each_with_index.filter_map do |row, index|
        error = row_error(row)
        "dose_occurrences[#{index}] #{error}" if error
      end
    end

    def self.facts(row)
      FIELDS.index_with do |field|
        value = row[field]
        %w[scheduled_at resolved_at].include?(field) && value.present? ? Time.iso8601(value).utc.iso8601(6) : value
      end
    end

    def self.conflicting?(record, row)
      current = DoseOccurrenceSerializer.new(record).as_json.with_indifferent_access
      facts(current) != facts(row)
    rescue ArgumentError, TypeError
      true
    end

    private

    def row_error(row)
      return 'has invalid outcome fields' unless DoseOccurrenceImportValidator.new(row).valid?
      return 'has a duplicate occurrence' unless @seen.add?(identity(row))
      return 'take is already linked to another occurrence' if conflicting_take_link?(row)

      record = @existing[row[:portable_id]] || @identities[identity(row)]
      return unless record && (record.portable_id != row[:portable_id] || self.class.conflicting?(record, row))

      'conflicts with existing history'
    end

    def existing_records
      scope = MedicationDoseOccurrence.where(household: @household)
      matching = scope.where(portable_id: @rows.pluck(:portable_id))
                      .or(scope.where(window_starts_on: @rows.pluck(:window_starts_on)))
                      .or(scope.where(medication_take_id: referenced_takes.select(:id)))
      matching.includes(:schedule, :person_medication, :medication_take).index_by(&:portable_id)
    end

    def referenced_takes
      MedicationTake.where(household: @household, portable_id: @rows.pluck(:medication_take_portable_id))
    end

    def existing_take_links
      @existing.values.filter_map do |record|
        next unless record.medication_take

        [record.medication_take.portable_id, identity(DoseOccurrenceSerializer.new(record).as_json)]
      end.to_h
    end

    def conflicting_take_link?(row)
      take_id = row[:medication_take_portable_id]
      return false if take_id.blank?

      linked_identity = @take_links[take_id]
      @take_links[take_id] ||= identity(row)
      linked_identity.present? && linked_identity != identity(row)
    end

    def identity(row)
      row.with_indifferent_access.values_at(:source_type, :source_portable_id, :window_starts_on, :position)
    end
  end
end
