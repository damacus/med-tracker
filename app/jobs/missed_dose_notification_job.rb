# frozen_string_literal: true

class MissedDoseNotificationJob < ApplicationJob
  queue_as :default

  GRACE_PERIOD = 30.minutes

  def perform(household_id, person_id, scheduled_on, scheduled_time)
    household = Household.find_by(id: household_id)
    return unless household

    TenantContext.with(account: nil, household: household) do
      deliver_missed_dose_notification(household, person_id, scheduled_on, scheduled_time)
    end
  end

  private

  def deliver_missed_dose_notification(household, person_id, scheduled_on, scheduled_time)
    person = Person.find_by(id: person_id, household: household)
    return unless eligible_person?(person)

    scheduled_at = scheduled_at(scheduled_on, scheduled_time)
    return unless missed_dose_due?(person, scheduled_time, scheduled_at)

    event = record_missed_dose_event(household, person, scheduled_on, scheduled_time)
    return unless event

    deliver_or_record_skip(event, household, person)
  end

  def missed_dose_due?(person, scheduled_time, scheduled_at)
    return false unless scheduled_at && Time.current >= scheduled_at + GRACE_PERIOD

    MedicationReminderEligibilityQuery.new(
      person: person,
      scheduled_time: scheduled_time,
      now: scheduled_at + GRACE_PERIOD
    ).medication_names.any?
  end

  def record_missed_dose_event(household, person, scheduled_on, scheduled_time)
    NotificationEvent.record_once!(
      household: household,
      person: person,
      event_type: 'missed_dose',
      event_key: "missed-dose:#{person.id}:#{scheduled_on}:#{scheduled_time}",
      metadata: { scheduled_on: scheduled_on, scheduled_time: scheduled_time }
    )
  end

  def deliver_or_record_skip(event, household, person)
    return record_skip(event, person, 'no_active_push_subscriptions') if person.account.push_subscriptions.none?

    PushNotificationService.send_to_account(
      person.account,
      title: 'Medication reminder',
      body: 'A dose may have been missed.',
      path: "/households/#{household.slug}/dashboard"
    )
    event.update!(sent_at: Time.current)
  end

  def eligible_person?(person)
    return false unless person&.account

    preference = person.notification_preference
    preference&.enabled && preference.missed_dose_enabled
  end

  def scheduled_at(scheduled_on, scheduled_time)
    date = Date.iso8601(scheduled_on.to_s)
    hour, min = scheduled_time.to_s.split(':', 3)
    return unless hour&.match?(/\A\d{1,2}\z/) && min&.match?(/\A\d{1,2}\z/)

    Time.zone.local(date.year, date.month, date.day, hour.to_i, min.to_i)
  rescue ArgumentError
    nil
  end

  def record_skip(event, person, reason)
    event.update!(skipped_reason: reason)
    Rails.logger.info("[MissedDoseNotificationJob] Skipped person_id=#{person.id} reason=#{reason}")
  end
end
