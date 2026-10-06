module PortableData
  class DoseOccurrenceImportWriter
    def initialize(household:, membership:, rows:)
      @household = household
      @membership = membership
      @rows = rows
    end

    def call
      preload_references
      @rows.each { |row| restore(row) }
    end

    private

    def preload_references
      ids = @rows.pluck(:source_portable_id)
      @sources = {
        'schedule' => Schedule.where(household: @household, portable_id: ids).index_by(&:portable_id),
        'person_medication' => PersonMedication.where(household: @household, portable_id: ids).index_by(&:portable_id)
      }
      @takes = MedicationTake.where(household: @household, portable_id: @rows.pluck(:medication_take_portable_id))
                             .index_by(&:portable_id)
      scope = MedicationDoseOccurrence.where(household: @household, portable_id: @rows.pluck(:portable_id))
      @existing = scope.includes(:schedule, :person_medication, :medication_take).index_by(&:portable_id)
    end

    def restore(row)
      return if replay?(row)

      record = MedicationDoseOccurrence.new(household: @household, portable_id: row[:portable_id])
      record.assign_attributes(row.slice(:window_starts_on, :window_ends_on, :position, :scheduled_at,
                                         :outcome, :reason, :note, :resolved_at))
      assign_references(record, row)
      record.save!
      @existing[record.portable_id] = record
    end

    def replay?(row)
      record = @existing[row[:portable_id]]
      return false unless record
      return true unless DoseOccurrenceImportPreflight.conflicting?(record, row)

      raise Importer::Error, 'Dose occurrence conflicts with existing history'
    end

    def assign_references(record, row)
      source = source_for(row)
      record.schedule = source if source.is_a?(Schedule)
      record.person_medication = source if source.is_a?(PersonMedication)
      record.medication_take = take_for(row)
      record.resolved_by_membership = @membership unless record.open?
    end

    def source_for(row)
      sources = @sources.fetch(row[:source_type]) { raise Importer::Error, 'Unsupported dose occurrence source type' }
      sources.fetch(row[:source_portable_id]) { raise Importer::Error, 'Dose occurrence source is unavailable' }
    end

    def take_for(row)
      return if row[:medication_take_portable_id].blank?

      @takes.fetch(row[:medication_take_portable_id]) { raise Importer::Error, 'Dose occurrence take is unavailable' }
    end
  end
end
