# frozen_string_literal: true

class ScheduleDailyRemindersJob < ApplicationJob
  queue_as :default

  PERIODS = NotificationPreference::PERIODS

  def perform
    Household.operational.find_each do |household|
      TenantContext.with(account: nil, household: household) do
        schedule_household_reminders(household)
      end
    end
  end

  private

  def schedule_household_reminders(household)
    scheduled_missed_person_ids = []
    reminder_preferences(household).find_each do |preference|
      schedule_preference(preference, scheduled_missed_person_ids)
    end
    enqueue_managed_missed_dose_reminders(household, scheduled_missed_person_ids)
  end

  def reminder_preferences(household)
    NotificationPreference.where(household:, enabled: true)
                          .where('dose_due_enabled = ? OR missed_dose_enabled = ?', true, true)
                          .includes(person: [:account, { schedules: %i[medication medication_takes] }])
  end

  def schedule_preference(preference, scheduled_missed_person_ids)
    return unless preference.person&.account

    enqueue_reminders_for(preference)
    scheduled_missed_person_ids << preference.person_id if preference.missed_dose_enabled
  rescue StandardError => e
    emit_job_failure(:unknown, e)
  end

  def enqueue_reminders_for(pref)
    period_reminder_times = pref.dose_due_enabled ? enqueue_period_reminders_for(pref) : []
    enqueue_schedule_time_reminders_for(pref, period_reminder_times)
  end

  def enqueue_period_reminders_for(pref)
    scheduled_times = []

    PERIODS.each do |period|
      time = pref.time_for_period(period)
      next unless time

      send_at = build_send_time(time)
      if send_at < Time.current
        record_past_occurrence(:dose_due)
        next
      end

      enqueue_period_reminder(pref, period, send_at)
      scheduled_times << send_at
    end

    scheduled_times
  end

  def enqueue_period_reminder(pref, period, send_at)
    enqueue_notification_job(
      MedicationReminderJob.new(pref.household_id, pref.person_id, period, nil, send_at),
      send_at:,
      kind: :dose_due
    )
  end

  def enqueue_schedule_time_reminders_for(pref, period_reminder_times)
    configured_times_for(pref.person).each do |time|
      enqueue_configured_time_for(pref, time, period_reminder_times)
    end
  end

  def enqueue_configured_time_for(pref, time, period_reminder_times)
    send_at = build_send_time_from_configured_time(time)
    return record_invalid_schedule(:unknown) if send_at.blank?
    return record_past_occurrence(schedule_kind(pref)) if send_at < Time.current

    enqueue_scheduled_dose_due(pref, send_at, time) if pref.dose_due_enabled && period_reminder_times.exclude?(send_at)
    enqueue_missed_dose_check_for(pref.person, send_at, time) if pref.missed_dose_enabled
  end

  def schedule_kind(pref)
    pref.missed_dose_enabled ? :missed_dose : :dose_due
  end

  def enqueue_scheduled_dose_due(pref, send_at, time)
    enqueue_notification_job(
      MedicationReminderJob.new(pref.household_id, pref.person_id, :scheduled, time, send_at),
      send_at:,
      kind: :dose_due
    )
  end

  def enqueue_managed_missed_dose_reminders(household, already_scheduled_person_ids)
    ManagedMissedDoseNotificationSubjectsQuery.new(household: household).call.each do |person|
      next if already_scheduled_person_ids.include?(person.id)

      enqueue_missed_dose_checks_for(person)
    rescue StandardError => e
      emit_job_failure(:missed_dose, e)
    end
  end

  def enqueue_missed_dose_checks_for(person)
    configured_times_for(person).each do |time|
      enqueue_missed_dose_time_for(person, time)
    end
  end

  def enqueue_missed_dose_time_for(person, time)
    send_at = build_send_time_from_configured_time(time)
    return record_invalid_schedule(:missed_dose) if send_at.blank?
    return record_past_occurrence(:missed_dose) if send_at < Time.current

    enqueue_missed_dose_check_for(person, send_at, time)
  end

  def enqueue_missed_dose_check_for(person, send_at, time)
    enqueue_notification_job(
      MissedDoseNotificationJob.new(person.household_id, person.id, send_at.to_date.iso8601, time),
      send_at: send_at + MissedDoseNotificationJob::GRACE_PERIOD,
      kind: :missed_dose
    )
  end

  def enqueue_notification_job(job, send_at:, kind:)
    context = Observability::CorrelationContext.start.next_attempt
    job.observability_context = context
    with_observability_context(context) do
      job.enqueue(wait_until: send_at)
      Observability::NotificationStage.emit(kind:, stage: :reminder_enqueue, reason: :enqueued)
    end
  rescue StandardError => e
    record_enqueue_failure(context, kind, e)
    raise
  end

  def record_enqueue_failure(context, kind, error)
    with_observability_context(context) do
      Observability::NotificationStage.emit(
        kind:,
        stage: :reminder_enqueue,
        reason: :job_failed,
        error_type: error.class
      )
    end
  end

  def emit_job_failure(kind, error)
    Observability::NotificationStage.emit(
      kind:,
      stage: :job_execution,
      reason: :job_failed,
      error_type: error.class
    )
  end

  def record_invalid_schedule(kind)
    Observability::NotificationStage.emit(
      kind:,
      stage: :reminder_enqueue,
      reason: :invalid_schedule
    )
  end

  def record_past_occurrence(kind)
    Observability::NotificationStage.emit(
      kind:,
      stage: :reminder_enqueue,
      reason: :past_occurrence
    )
  end

  def with_observability_context(context)
    previous = Current.observability_context
    Current.observability_context = context
    yield
  ensure
    Current.observability_context = previous
  end

  def configured_times_for(person)
    MedicationReminderEligibilityQuery.new(person: person).configured_times
  end

  def build_send_time(time)
    today = Time.zone.today
    Time.zone.local(today.year, today.month, today.day, time.hour, time.min)
  end

  def build_send_time_from_configured_time(time)
    hour, min = time.to_s.split(':', 3)
    return unless hour&.match?(/\A\d{1,2}\z/) && min&.match?(/\A\d{1,2}\z/)

    today = Time.zone.today
    Time.zone.local(today.year, today.month, today.day, hour.to_i, min.to_i)
  rescue ArgumentError
    nil
  end
end
