# frozen_string_literal: true

module PersonViewable
  extend ActiveSupport::Concern

  private

  def person_show_view(person)
    schedules = person.schedules.includes(:medication, :dosage)
    person_medications = person.person_medications.includes(:medication).ordered
    today_start = Time.current.beginning_of_day

    takes_by_schedule = MedicationTake
                        .where(schedule_id: schedules.map(&:id), taken_at: today_start..)
                        .order(taken_at: :desc)
                        .group_by(&:schedule_id)

    takes_by_person_medication = MedicationTake
                                 .where(person_medication_id: person_medications.map(&:id), taken_at: today_start..)
                                 .order(taken_at: :desc)
                                 .group_by(&:person_medication_id)

    Components::People::ShowView.new(
      person: person,
      schedules: schedules,
      person_medications: person_medications,
      preloaded_takes: {
        schedules: takes_by_schedule,
        person_medications: takes_by_person_medication
      },
      current_user: current_user
    )
  end
end
