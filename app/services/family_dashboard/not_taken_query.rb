module FamilyDashboard
  class NotTakenQuery
    def initialize(schedules:, person_medications: [])
      @schedules = schedules
      @person_medications = person_medications.select(&:routine?)
    end

    def for_source(source)
      outcomes_by_source.fetch(source_key(source), [])
    end

    def by_person(people)
      outcomes = outcomes_by_source.values.flatten.group_by { |outcome| outcome.source.person_id }
      people.index_with { |person| outcomes.fetch(person.id, []) }
    end

    private

    def outcomes_by_source
      @outcomes_by_source ||= records.includes(schedule: :medication, person_medication: :medication)
                                     .select { |outcome| outcome.window_starts_on == window_start(outcome.source) }
                                     .group_by { |outcome| source_key(outcome.source) }
    end

    def records
      earliest_window = @person_medications.map { |source| window_start(source) }.min || Date.current
      MedicationDoseOccurrence.where(outcome: 'not_taken').where(
        schedule_id: @schedules.map(&:id), window_starts_on: Date.current
      ).or(MedicationDoseOccurrence.where(outcome: 'not_taken', person_medication_id: @person_medications.map(&:id),
                                          window_starts_on: earliest_window..Date.current))
    end

    def window_start(source)
      return Date.current if source.is_a?(Schedule)

      DoseCycle.new(source.dose_cycle).range_for(Time.current).begin.to_date
    end

    def source_key(source) = [source.class.name, source.id]
  end
end
