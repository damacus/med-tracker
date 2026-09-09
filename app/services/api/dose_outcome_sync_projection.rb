module Api
  class DoseOutcomeSyncProjection
    RECORD_TYPE = 'MedicationDoseOccurrence'.freeze

    def initialize(household:, people_scope:)
      @household = household
      @people_scope = people_scope
    end

    def events(scope)
      scope.where.not(record_type: RECORD_TYPE)
           .or(scope.where(record_type: RECORD_TYPE, record_portable_id: occurrences.select(:portable_id)))
    end

    def tombstones(scope)
      permitted = scope.where(record_type: RECORD_TYPE)
                       .where("metadata ->> 'person_portable_id' IN (?)", @people_scope.pluck(:portable_id))
      scope.where.not(record_type: RECORD_TYPE).or(permitted)
    end

    def payloads(events)
      ids = events.select { |event| event.record_type == RECORD_TYPE }.map(&:record_portable_id)
      occurrences.where(portable_id: ids).includes(:schedule, :person_medication, :medication_take).to_h do |record|
        [record.portable_id, PortableData::DoseOccurrenceSerializer.new(record).as_json]
      end
    end

    private

    def occurrences
      scope = MedicationDoseOccurrence.where(household: @household)
      schedules = Schedule.where(household: @household, person_id: @people_scope.select(:id))
      assignments = PersonMedication.where(household: @household, person_id: @people_scope.select(:id))
      scope.where(schedule_id: schedules.select(:id)).or(scope.where(person_medication_id: assignments.select(:id)))
    end
  end
end
