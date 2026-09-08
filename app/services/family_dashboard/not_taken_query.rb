module FamilyDashboard
  class NotTakenQuery
    def initialize(schedules:)
      @schedules = schedules
    end

    def for_source(source)
      source.is_a?(Schedule) ? outcomes_by_schedule.fetch(source.id, []) : []
    end

    def by_person(people)
      outcomes = outcomes_by_schedule.values.flatten.group_by { |outcome| outcome.schedule.person_id }
      people.index_with { |person| outcomes.fetch(person.id, []) }
    end

    private

    def outcomes_by_schedule
      @outcomes_by_schedule ||= begin
        records = MedicationDoseOccurrence.where(
          schedule_id: @schedules.map(&:id), window_starts_on: Date.current, outcome: 'not_taken'
        )
        records.includes(schedule: :medication).to_a.group_by(&:schedule_id)
      end
    end
  end
end
