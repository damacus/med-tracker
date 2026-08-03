# frozen_string_literal: true

class LowStockNotificationJob < ApplicationJob
  queue_as :notifications

  def perform(household_id, medication_id, take_id)
    household = Household.operational.find_by(id: household_id)
    return record_outcome(:household_unavailable, stage: :low_stock_evaluation) unless household

    TenantContext.with(account: nil, household: household) do
      medication = find_medication(medication_id, household)
      return record_outcome(:no_medications, stage: :low_stock_evaluation) unless medication

      eligible_people = eligible_people_for(medication)
      record_outcome(:eligible, stage: :low_stock_evaluation, recipient_count: eligible_people.size)
      eligible_people.each do |person|
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
    return record_outcome(:duplicate, stage: :notification_intent) unless event

    record_outcome(:intent_recorded, stage: :notification_intent)

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
    return record_skip(event, 'no_active_push_subscriptions') if person.account.push_subscriptions.none?

    record_outcome(:attempted, stage: :recipient_attempt)
    PushNotificationService.send_to_account(
      person.account,
      title: 'Stock reminder',
      body: 'A medication may be running low.',
      path: "/households/#{medication.household.slug}/dashboard",
      notification_kind: :low_stock
    )
    event.update!(sent_at: Time.current)
  end

  def record_skip(event, reason)
    event.update!(skipped_reason: reason)
    record_outcome(reason.to_sym, stage: :low_stock_delivery)
  end

  def record_outcome(reason, stage:, recipient_count: nil)
    Observability::NotificationStage.emit(
      kind: :low_stock,
      stage:,
      reason:,
      recipient_count:
    )
  end
end
