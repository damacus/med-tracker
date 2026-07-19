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

    NotificationPreference.where(household: household, enabled: true)
                          .where('dose_due_enabled = ? OR missed_dose_enabled = ?', true, true)
                          .includes(person: [:account, { schedules: %i[medication medication_takes] }])
                          .find_each do |pref|
      next unless pref.person&.account

      enqueue_reminders_for(pref)
      scheduled_missed_person_ids << pref.person_id if pref.missed_dose_enabled
    rescue StandardError => e
      Rails.logger.error("Failed to schedule reminders for preference #{pref.id}: #{e.class}: #{e.message}")
    end

    enqueue_managed_missed_dose_reminders(household, scheduled_missed_person_ids)
  end

  def enqueue_reminders_for(pref)
    enqueue_period_reminders_for(pref) if pref.dose_due_enabled
    enqueue_schedule_time_reminders_for(pref)
  end

  def enqueue_period_reminders_for(pref)
    PERIODS.each do |period|
      time = pref.time_for_period(period)
      next unless time

      send_at = build_send_time(time)
      next if send_at < Time.current

      MedicationReminderJob.set(wait_until: send_at).perform_later(pref.household_id, pref.person_id, period)
    end
  end

  def enqueue_schedule_time_reminders_for(pref)
    configured_times_for(pref.person).each do |time|
      send_at = build_send_time_from_configured_time(time)
      next if send_at.blank? || send_at < Time.current

      if pref.dose_due_enabled
        MedicationReminderJob
          .set(wait_until: send_at)
          .perform_later(pref.household_id, pref.person_id, :scheduled, time)
      end
      enqueue_missed_dose_check_for(pref.person, send_at, time) if pref.missed_dose_enabled
    end
  end

  def enqueue_managed_missed_dose_reminders(household, already_scheduled_person_ids)
    ManagedMissedDoseNotificationSubjectsQuery.new(household: household).call.each do |person|
      next if already_scheduled_person_ids.include?(person.id)

      enqueue_missed_dose_checks_for(person)
    rescue StandardError => e
      Rails.logger.error("Failed to schedule managed reminders for person #{person.id}: #{e.class}: #{e.message}")
    end
  end

  def enqueue_missed_dose_checks_for(person)
    configured_times_for(person).each do |time|
      send_at = build_send_time_from_configured_time(time)
      next if send_at.blank? || send_at < Time.current

      enqueue_missed_dose_check_for(person, send_at, time)
    end
  end

  def enqueue_missed_dose_check_for(person, send_at, time)
    MissedDoseNotificationJob
      .set(wait_until: send_at + MissedDoseNotificationJob::GRACE_PERIOD)
      .perform_later(person.household_id, person.id, send_at.to_date.iso8601, time)
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
