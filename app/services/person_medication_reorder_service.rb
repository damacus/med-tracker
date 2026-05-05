# frozen_string_literal: true

class PersonMedicationReorderService
  Result = Data.define(:success, :person_medication) do
    def success?
      success
    end
  end

  def call(person_medication:, direction:)
    adjacent = adjacent_record(person_medication, direction)
    return Result.new(success: false, person_medication: person_medication) unless adjacent

    swap_positions(person_medication, adjacent)
    Result.new(success: true, person_medication: person_medication)
  end

  private

  def adjacent_record(person_medication, direction)
    case direction
    when 'up'
      person_medication.person.person_medications
                       .where(position: ...person_medication.position)
                       .order(position: :desc, id: :desc)
                       .first
    when 'down'
      person_medication.person.person_medications
                       .where(position: (person_medication.position + 1)..)
                       .order(position: :asc, id: :asc)
                       .first
    end
  end

  def swap_positions(person_medication, adjacent)
    original_position = person_medication.position

    person_medication.transaction do
      person_medication.update!(position: adjacent.position)
      adjacent.update!(position: original_position)
    end
  end
end
