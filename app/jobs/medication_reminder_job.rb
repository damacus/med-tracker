# frozen_string_literal: true

class MedicationReminderJob < ApplicationJob
  queue_as :notifications

  PERIOD_LABELS = {
    morning: 'Morning',
    afternoon: 'Afternoon',
    evening: 'Evening',
    night: 'Night'
  }.freeze

  def perform(household_id, person_id, period, scheduled_time = nil, intended_at = nil)
    household = Household.operational.find_by(id: household_id)
    return record_outcome(:household_unavailable) unless household

    event = TenantContext.with(account: nil, household: household) do
      prepare_reminder(household, person_id, period, scheduled_time, intended_at)
    end
    return unless event

    TenantContext.with(account: nil, household: household) { deliver_recorded_reminder(event) }
  end

  private

  def prepare_reminder(household, person_id, period, scheduled_time, intended_at)
    return unless reminder_context_available?(household, person_id, period, scheduled_time, intended_at)

    med_names = eligible_medication_names(scheduled_time)
    return unless med_names

    reserve_reminder(med_names)
  end

  def reminder_context_available?(household, person_id, period, scheduled_time, intended_at)
    @person = Person.find_by(id: person_id, household: household)
    return record_unavailable(:person_unavailable) unless @person&.account

    @household = household
    @pref = @person.notification_preference
    return record_unavailable(:preference_disabled) unless @pref&.enabled && @pref.dose_due_enabled

    @scheduled_time = scheduled_time
    @period = period
    @intended_at = intended_occurrence_at(intended_at)
    return record_unavailable(:invalid_occurrence) unless @intended_at

    preserve_legacy_occurrence_for_retry

    true
  end

  def eligible_medication_names(scheduled_time)
    med_names = MedicationReminderEligibilityQuery.new(person: @person, scheduled_time: scheduled_time).medication_names
    return record_unavailable(:no_medications) if med_names.empty?

    record_outcome(:eligible)
    med_names
  end

  def reserve_reminder(med_names)
    event = record_due_event
    return record_unavailable(:duplicate) unless event

    @med_names = med_names
    event
  end

  def record_unavailable(reason)
    record_outcome(reason)
    nil
  end

  def deliver_recorded_reminder(event)
    send_push_notification(@med_names)
    event.update!(sent_at: Time.current)
  end

  def record_due_event
    timestamp = canonical_occurrence_timestamp
    NotificationEvent.record_once!(
      household: @household,
      person: @person,
      event_type: 'dose_due',
      event_key: "dose-due:#{@person.id}:#{timestamp}",
      metadata: { intended_at: timestamp, delivery_status: 'delivery_unknown' }
    )
  end

  def send_push_notification(med_names)
    Observability::NotificationStage.emit(
      kind: :dose_due,
      stage: :recipient_attempt,
      reason: :attempted
    )
    PushNotificationService.send_to_account(
      @person.account,
      title: notification_title,
      body: notification_body(med_names),
      path: "/households/#{@household.slug}/dashboard",
      notification_kind: :dose_due
    )
  end

  def record_outcome(reason)
    Observability::NotificationStage.emit(
      kind: :dose_due,
      stage: :reminder_eligibility,
      reason:
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

  def intended_occurrence_at(explicit_intended_at)
    occurrence = explicit_intended_at.presence || scheduled_at || legacy_occurrence_at
    normalize_occurrence(occurrence)
  end

  def preserve_legacy_occurrence_for_retry
    arguments[4] = @intended_at if arguments.length < 5
  end

  def legacy_occurrence_at
    hour, min = time_components(@scheduled_time.presence || @pref.time_for_period(@period))
    return unless hour && min

    today = Time.zone.today
    Time.zone.local(today.year, today.month, today.day, hour, min)
  end

  def time_components(value)
    return [value.hour, value.min] if value.respond_to?(:hour) && value.respond_to?(:min)

    match = value.to_s.match(/\A(\d{1,2}):(\d{1,2})(?::.*)?\z/)
    return unless match

    hour = match[1].to_i
    min = match[2].to_i
    return unless hour.between?(0, 23) && min.between?(0, 59)

    [hour, min]
  end

  def normalize_occurrence(occurrence)
    return unless occurrence

    time = occurrence.respond_to?(:in_time_zone) ? occurrence.in_time_zone : Time.zone.parse(occurrence.to_s)
    time.change(sec: 0)
  rescue ArgumentError, TypeError
    nil
  end

  def canonical_occurrence_timestamp
    @intended_at.utc.iso8601
  end
end
