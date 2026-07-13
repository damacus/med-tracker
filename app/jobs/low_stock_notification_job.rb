# frozen_string_literal: true

class LowStockNotificationJob < ApplicationJob
  queue_as :default

  def perform(household_id, medication_id, take_id)
    household = Household.operational.find_by(id: household_id)
    return unless household

    TenantContext.with(account: nil, household: household) do
      medication = find_medication(medication_id, household)
      return unless medication

      eligible_people_for(medication).each do |person|
        deliver_low_stock_notification(medication, person, take_id)
      end
    end
  end

  private

  def find_medication(medication_id, household)
    Medication.includes(schedules: { person: %i[account notification_preference] },
                        person_medications: { person: %i[account notification_preference] })
              .find_by(id: medication_id, household: household)
  end

  def eligible_people_for(medication)
    (medication.schedules.active.map(&:person) + medication.person_medications.active.map(&:person))
      .compact
      .uniq
      .select { |person| eligible_person?(person) }
  end

  def eligible_person?(person)
    return false unless person.account

    preference = person.notification_preference
    preference&.enabled && preference.low_stock_enabled
  end

  def deliver_low_stock_notification(medication, person, take_id)
    event = record_low_stock_event(medication, person, take_id)
    return unless event

    deliver_or_record_skip(event, medication, person)
  end

  def record_low_stock_event(medication, person, take_id)
    NotificationEvent.record_once!(
      household: medication.household,
      person: person,
      event_type: 'low_stock',
      event_key: "low-stock:#{medication.id}:#{take_id}:#{person.id}",
      metadata: { medication_id: medication.id, take_id: take_id }
    )
  end

  def deliver_or_record_skip(event, medication, person)
    return record_skip(event, person, 'no_active_push_subscriptions') if person.account.push_subscriptions.none?

    PushNotificationService.send_to_account(
      person.account,
      title: 'Stock reminder',
      body: 'A medication may be running low.',
      path: "/households/#{medication.household.slug}/dashboard"
    )
    event.update!(sent_at: Time.current)
  end

  def record_skip(event, person, reason)
    event.update!(skipped_reason: reason)
    Rails.logger.info("[LowStockNotificationJob] Skipped person_id=#{person.id} reason=#{reason}")
  end
end
