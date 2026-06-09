# frozen_string_literal: true

module FamilyDashboard
  class TaskPriority
    STATUS_PRIORITIES = {
      upcoming: 0,
      available: 0,
      cooldown: 1,
      out_of_stock: 2,
      taken: 3,
      max_reached: 3
    }.freeze

    def initialize(reference_time: Time.current)
      @reference_time = reference_time
    end

    def sort_rows(rows)
      rows.sort_by { |row| row_sort_key(row) }
    end

    def sort_people(people, rows)
      rows_by_person = rows.group_by { |row| row[:person] }

      people.select { |person| rows_by_person.fetch(person, []).any? }
            .sort_by { |person| person_sort_key(person, rows_by_person.fetch(person, [])) }
    end

    private

    attr_reader :reference_time

    def person_sort_key(person, rows)
      [rows.map { |row| row_sort_key(row) }.min, person.id]
    end

    def row_sort_key(row)
      [
        STATUS_PRIORITIES.fetch(row[:status], 4),
        row_relevant_time(row),
        row[:person].id,
        row[:source].class.name,
        row[:source].id
      ]
    end

    def row_relevant_time(row)
      return upcoming_relevant_time(row) if %i[upcoming available].include?(row[:status])
      return cooldown_relevant_time(row) if row[:status] == :cooldown

      fallback_relevant_time(row)
    end

    def upcoming_relevant_time(row) = row[:scheduled_at] || reference_time

    def cooldown_relevant_time(row)
      row[:scheduled_at] || row[:source].next_available_time || reference_time.end_of_day
    end

    def fallback_relevant_time(row)
      row[:scheduled_at] || row[:taken_at] || reference_time.end_of_day
    end
  end
end
