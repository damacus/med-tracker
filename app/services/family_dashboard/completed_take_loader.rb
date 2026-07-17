# frozen_string_literal: true

module FamilyDashboard
  class CompletedTakeLoader
    INCLUDES = [
      :taken_from_location,
      :taken_from_medication,
      { schedule: %i[person medication], person_medication: %i[person medication] }
    ].freeze

    def initialize(people:, date:)
      @people = people
      @date = date
    end

    def call
      takes_by_person = people.index_with { [] }
      people_by_id = people.index_by(&:id)
      fetch_takes.each do |take|
        person = people_by_id[take.person&.id]
        takes_by_person.fetch(person) << take if person
      end
      takes_by_person
    end

    private

    attr_reader :people, :date

    def fetch_takes
      scheduled = MedicationTake.where(taken_at: date.all_day, schedule_id: schedule_ids)
      direct = MedicationTake.where(taken_at: date.all_day, person_medication_id: person_medication_ids)
      scheduled.or(direct).includes(*INCLUDES).to_a
    end

    def schedule_ids
      Schedule.where(person_id: people.map(&:id)).select(:id)
    end

    def person_medication_ids
      PersonMedication.where(person_id: people.map(&:id)).select(:id)
    end
  end
end
