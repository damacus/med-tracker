# frozen_string_literal: true

module FamilyDashboard
  class AdministrationSourceLoader
    Result = Data.define(:schedules_by_person, :person_medications_by_person)

    def initialize(people:, date:, reference_time:, include_paused:)
      @people = people
      @date = date
      @reference_time = reference_time
      @include_paused = include_paused
    end

    def call
      Result.new(
        schedules_by_person: grouped_schedules,
        person_medications_by_person: grouped_person_medications
      )
    end

    private

    attr_reader :people, :date, :reference_time, :include_paused

    def grouped_schedules
      scope = unretired_at(Schedule.where(start_date: ..date, end_date: date..))
      group_for_people(active_if_required(scope).includes(:medication))
    end

    def grouped_person_medications
      scope = unretired_at(PersonMedication.all)
      group_for_people(active_if_required(scope).includes(:medication))
    end

    def unretired_at(scope)
      retired_at = scope.klass.arel_table[:retired_at]
      scope.where(retired_at.eq(nil).or(retired_at.gt(reference_time)))
    end

    def active_if_required(scope)
      include_paused ? scope : scope.where(active: true)
    end

    def group_for_people(scope)
      grouped = scope.where(person_id: people.map(&:id)).to_a.group_by(&:person_id)
      people.to_h { |person| [person.id, grouped.fetch(person.id, [])] }
    end
  end
end
