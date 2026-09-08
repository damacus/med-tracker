module Api
  class MedicationPausePeriodVisibility
    def initialize(household:, person_scope:)
      @household = household
      @person_scope = person_scope
    end

    def periods
      scope = MedicationPausePeriod.where(household:)
      scope.where(schedule_id: sources(Schedule).select(:id))
           .or(scope.where(person_medication_id: sources(PersonMedication).select(:id)))
    end

    def sources(klass)
      klass.where(household:, person_id: person_scope.select(:id))
    end

    private

    attr_reader :household, :person_scope
  end
end
