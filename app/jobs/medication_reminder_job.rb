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
    household = Household.operational.find_by(id: household_id)
    return unless household

    TenantContext.with(account: nil, household: household) do
      deliver_reminder(household, person_id, period, scheduled_time)
    end
  end

  private

  def deliver_reminder(household, person_id, period, scheduled_time)
    @person = Person.find_by(id: person_id, household: household)
    return unless @person&.account

    @household = household
    @pref = @person.notification_preference
    return unless @pref&.enabled && @pref.dose_due_enabled

    @scheduled_time = scheduled_time
    @period = period

    med_names = MedicationReminderEligibilityQuery.new(person: @person, scheduled_time: scheduled_time).medication_names
    return if med_names.empty?

    send_push_notification(med_names)
  end

  def send_push_notification(med_names)
    PushNotificationService.send_to_account(
      @person.account,
      title: notification_title,
      body: notification_body(med_names),
      path: "/households/#{@household.slug}/dashboard"
    )
  end

  def notification_title
    @pref.private_text_enabled ? 'Medication reminder' : 'Medication Reminder'
  end

  def notification_body(med_names)
    return 'A dose is due.' if @pref.private_text_enabled

    period_label = @scheduled_time.presence || PERIOD_LABELS[@period.to_sym] || @period.to_s.humanize
    "#{period_label} medications: #{med_names.join(', ')}"
  end
end
