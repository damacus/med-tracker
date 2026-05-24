# frozen_string_literal: true

class MedicationReminderJob < ApplicationJob
  queue_as :default

  PERIOD_LABELS = {
    morning: 'Morning',
    afternoon: 'Afternoon',
    evening: 'Evening',
    night: 'Night'
  }.freeze

  def perform(person_id, period, scheduled_time = nil)
    person = Person.find_by(id: person_id)
    return unless person
    return unless person.account

    med_names = MedicationReminderEligibilityQuery.new(person: person, scheduled_time: scheduled_time).medication_names
    return if med_names.empty?

    period_label = scheduled_time.presence || PERIOD_LABELS[period.to_sym] || period.to_s.humanize
    body = "#{period_label} medications: #{med_names.join(', ')}"

    PushNotificationService.send_to_account(
      person.account,
      title: 'Medication Reminder',
      body: body,
      path: '/'
    )
  end
end
