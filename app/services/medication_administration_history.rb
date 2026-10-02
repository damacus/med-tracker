# frozen_string_literal: true

class MedicationAdministrationHistory
  def self.exists_for?(record) = new(record).exists?

  def initialize(record)
    @record = record
  end

  def exists?
    return true if record.is_a?(Location) && MedicationTake.exists?(taken_from_location_id: record.id)

    schedules, person_medications = administration_sources
    scheduled_takes = MedicationTake.where(schedule_id: schedules.select(:id))
    ad_hoc_takes = MedicationTake.where(person_medication_id: person_medications.select(:id))
    scheduled_takes.or(ad_hoc_takes).exists? || saved_outcomes?(schedules, person_medications)
  end

  private

  attr_reader :record

  def saved_outcomes?(schedules, person_medications)
    scheduled = MedicationDoseOccurrence.where(schedule_id: schedules.select(:id))
    routine = MedicationDoseOccurrence.where(person_medication_id: person_medications.select(:id))
    scheduled.or(routine).exists?
  end

  def administration_sources
    case record
    when Medication, Person
      [record.schedules, record.person_medications]
    when Location
      medication_ids = record.medications.select(:id)
      [Schedule.where(medication_id: medication_ids), PersonMedication.where(medication_id: medication_ids)]
    else
      raise ArgumentError, "Unsupported administration history owner: #{record.class}"
    end
  end
end
