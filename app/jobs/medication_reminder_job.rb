# frozen_string_literal: true

class MedicationReminderJob < ApplicationJob
  queue_as :default

  PERIOD_LABELS = {
    morning: 'Morning',
    afternoon: 'Afternoon',
    evening: 'Evening',
    night: 'Night'
  }.freeze

  def perform(household_id, person_id, period, scheduled_time = nil)
    household = Household.find_by(id: household_id)
    return unless household

    TenantContext.with(account: nil, household: household) do
      deliver_reminder(household, person_id, period, scheduled_time)
    end
  end

  private

  def deliver_reminder(household, person_id, period, scheduled_time)
    person = Person.find_by(id: person_id, household: household)
    return unless person&.account

    return if medication_names(person, scheduled_time).empty?

    PushNotificationService.send_to_account(
      person.account,
      title: 'Medication Reminder',
      body: reminder_body(period, scheduled_time),
      path: "/households/#{household.slug}/dashboard"
    )
  end

  def medication_names(person, scheduled_time)
    MedicationReminderEligibilityQuery.new(person: person, scheduled_time: scheduled_time).medication_names
  end

  def reminder_body(period, scheduled_time)
    period_label = scheduled_time.presence || PERIOD_LABELS[period.to_sym] || period.to_s.humanize

    "#{period_label} medication reminder. Open MedTracker for details."
  end
end
