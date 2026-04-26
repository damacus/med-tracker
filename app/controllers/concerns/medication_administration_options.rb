# frozen_string_literal: true

module MedicationAdministrationOptions
  extend ActiveSupport::Concern

  private

  def administration_schedules
    policy_scope(Schedule)
      .includes(:person, :medication)
      .where(medication: @medication)
      .active
      .select { |schedule| policy(schedule).take_medication? }
      .sort_by { |schedule| [schedule.person.name, schedule.id] }
  end

  def administration_person_medications
    policy_scope(PersonMedication)
      .includes(:person, :medication)
      .where(medication: @medication)
      .select { |person_medication| policy(person_medication).take_medication? }
      .sort_by { |person_medication| [person_medication.person.name, person_medication.id] }
  end
end
