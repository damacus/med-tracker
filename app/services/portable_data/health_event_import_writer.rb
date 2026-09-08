module PortableData
  class HealthEventImportWriter
    def initialize(household:, rows:)
      @household = household
      @rows = rows
    end

    def call
      preload_references
      @rows.each { |row| restore(row) }
    end

    private

    def preload_references
      @people = Person.where(household: @household,
                             portable_id: @rows.pluck(:person_portable_id)).index_by(&:portable_id)
      preload_medications
      @events = HealthEvent.where(household: @household, portable_id: @rows.pluck(:portable_id))
                           .includes(:medications).index_by(&:portable_id)
    end

    def preload_medications
      medication_ids = @rows.flat_map { |row| Array(row[:medication_portable_ids]) }.uniq
      @medications = Medication.where(household: @household, portable_id: medication_ids).index_by(&:portable_id)
    end

    def restore(row)
      event = @events[row[:portable_id]] || HealthEvent.new(household: @household, portable_id: row[:portable_id])
      event.assign_attributes(row.slice(:event_kind, :severity, :title, :notes, :started_on, :ended_on))
      event.person = person_for(row)
      event.medications = medications_for(row)
      event.save!
    end

    def person_for(row)
      @people.fetch(row[:person_portable_id]) { raise Importer::Error, 'Health event person is unavailable' }
    end

    def medications_for(row)
      Array(row[:medication_portable_ids]).map do |portable_id|
        @medications.fetch(portable_id) { raise Importer::Error, 'Health event medication is unavailable' }
      end
    end
  end
end
