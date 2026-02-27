# frozen_string_literal: true

class MedicationReminderJob < ApplicationJob
  queue_as :default

  PERIOD_LABELS = {
    morning: 'Morning',
    afternoon: 'Afternoon',
    evening: 'Evening',
    night: 'Night'
  }.freeze

  def perform(person_id, period)
    person = Person.find_by(id: person_id)
    return unless person
    return unless person.account

    med_names = active_medication_names(person)
    return if med_names.empty?

    period_label = PERIOD_LABELS[period.to_sym] || period.to_s.humanize
    body = "#{period_label} medications: #{med_names.join(', ')}"

    PushNotificationService.send_to_account(
      person.account,
      title: 'Medication Reminder',
      body: body,
      path: '/'
    )
  end

  private

  def active_medication_names(person)
    schedule_meds = person.schedules.active.map(&:medication_name)
    person_meds = person.person_medications.includes(:medication).map { |pm| pm.medication.name }
    (schedule_meds + person_meds).uniq
  end
end
