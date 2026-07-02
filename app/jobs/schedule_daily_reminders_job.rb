# frozen_string_literal: true

class ScheduleDailyRemindersJob < ApplicationJob
  queue_as :default

  PERIODS = NotificationPreference::PERIODS

  def perform
    Household.active.find_each do |household|
      TenantContext.with(account: nil, household: household) do
        schedule_household_reminders(household)
      end
    end
  end

  private

  def schedule_household_reminders(household)
    NotificationPreference.where(household: household, enabled: true)
                          .where('dose_due_enabled = ? OR missed_dose_enabled = ?', true, true)
                          .includes(person: [:account, { schedules: %i[medication medication_takes] }])
                          .find_each do |pref|
      next unless pref.person&.account

      enqueue_reminders_for(pref)
    rescue StandardError => e
      Rails.logger.error("Failed to schedule reminders for preference #{pref.id}: #{e.class}: #{e.message}")
    end
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
    configured_times_for(pref).each do |time|
      send_at = build_send_time_from_configured_time(time)
      next if send_at.blank? || send_at < Time.current

      if pref.dose_due_enabled
        MedicationReminderJob
          .set(wait_until: send_at)
          .perform_later(pref.household_id, pref.person_id, :scheduled, time)
      end
      enqueue_missed_dose_check_for(pref, send_at, time) if pref.missed_dose_enabled
    end
  end

  def enqueue_missed_dose_check_for(pref, send_at, time)
    MissedDoseNotificationJob
      .set(wait_until: send_at + MissedDoseNotificationJob::GRACE_PERIOD)
      .perform_later(pref.household_id, pref.person_id, send_at.to_date.iso8601, time)
  end

  def configured_times_for(pref)
    times = []
    times.concat(MedicationReminderEligibilityQuery.new(person: pref.person).configured_times) if pref.dose_due_enabled
    times.concat(active_schedule_times_for(pref.person)) if pref.missed_dose_enabled
    times.uniq
  end

  def active_schedule_times_for(person)
    active_schedules_for(person)
      .reject(&:schedule_type_prn?)
      .select { |schedule| schedule.applies_on?(Time.zone.today) }
      .flat_map { |schedule| configured_times_for_schedule(schedule) }
      .uniq
  end

  def active_schedules_for(person)
    if person.schedules.loaded?
      person.schedules.select(&:active?)
    else
      person.schedules.active.to_a
    end
  end

  def configured_times_for_schedule(schedule)
    config = schedule.schedule_config.to_h
    Array(config['times'] || config[:times]).compact_blank
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
