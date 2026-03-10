# frozen_string_literal: true

module PersonViewable
  extend ActiveSupport::Concern

  private

  def person_show_view(person, editing: false)
    schedules = person.schedules.includes(:medication, :dosage)
    person_medications = person.person_medications.includes(:medication).ordered

    Components::People::ShowView.new(
      person: person,
      schedules: schedules,
      person_medications: person_medications,
      editing: editing,
      preloaded_takes: fetch_preloaded_takes(schedules, person_medications),
      stock_by_criteria: fetch_stock_by_criteria,
      current_user: current_user
    )
  end

  def fetch_preloaded_takes(schedules, person_medications)
    today_start = Time.current.beginning_of_day

    schedules_takes = MedicationTake.where(schedule_id: schedules.map(&:id), taken_at: today_start..)
                                    .order(taken_at: :desc)
                                    .group_by(&:schedule_id)

    pm_takes = MedicationTake.where(person_medication_id: person_medications.map(&:id), taken_at: today_start..)
                             .order(taken_at: :desc)
                             .group_by(&:person_medication_id)

    {
      schedules: schedules_takes,
      person_medications: pm_takes
    }
  end

  def fetch_stock_by_criteria
    scope = MedicationPolicy::Scope.new(current_user, Medication.all).resolve
    all_accessible = scope.joins(:location)
                          .includes(:location)
                          .order('locations.name ASC, medications.id ASC')
                          .to_a

    all_accessible.group_by do |m|
      [m.name, m.dosage_amount, m.dosage_unit]
    end
  end
end
