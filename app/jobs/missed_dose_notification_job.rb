# frozen_string_literal: true

class MissedDoseNotificationJob < ApplicationJob
  queue_as :default

  GRACE_PERIOD = 30.minutes

  def perform(household_id, person_id, scheduled_on, scheduled_time)
    household = Household.operational.find_by(id: household_id)
    return unless household

    TenantContext.with(account: nil, household: household) do
      deliver_missed_dose_notification(household, person_id, scheduled_on, scheduled_time)
    end
  end

  private

  def deliver_missed_dose_notification(household, person_id, scheduled_on, scheduled_time)
    person = Person.find_by(id: person_id, household: household)
    return unless person

    recipients = MissedDoseNotificationRecipientsQuery.new(person: person).call
    return if recipients.empty?

    scheduled_at = parsed_scheduled_at(scheduled_on, scheduled_time)
    return unless missed_dose_due?(person, scheduled_time, scheduled_at)

    event = record_missed_dose_event(household, person, scheduled_on, scheduled_time)
    return unless event

    deliver_or_record_skip(event, household, person, recipients)
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

  def deliver_or_record_skip(event, household, person, recipients)
    active_recipients = recipients.select { |recipient| active_push_recipient?(recipient) }
    return record_skip(event, person, 'no_active_push_subscriptions') if active_recipients.empty?

    active_recipients.each do |recipient|
      PushNotificationService.send_to_account(
        recipient.account,
        title: 'Medication reminder',
        body: notification_body(person, recipient),
        path: "/households/#{household.slug}/dashboard"
      )
    end
    event.update!(sent_at: Time.current)
  end

  def active_push_recipient?(recipient)
    recipient.account.push_subscriptions.exists? || recipient.account.native_device_tokens.exists?
  end

  def notification_body(person, recipient)
    return 'A dose may have been missed.' unless recipient.managed && !recipient.preference.private_text_enabled

    "#{person.name} may have missed a dose."
  end

  def parsed_scheduled_at(scheduled_on, scheduled_time)
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
