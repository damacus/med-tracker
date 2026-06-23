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
                          .includes(person: [:account, { schedules: %i[medication medication_takes] }])
                          .find_each do |pref|
      next unless pref.person&.account

      enqueue_reminders_for(pref)
    rescue StandardError => e
      Rails.logger.error("Failed to schedule reminders for preference #{pref.id}: #{e.class}: #{e.message}")
    end
  end

  def enqueue_reminders_for(pref)
    enqueue_period_reminders_for(pref)
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

      MedicationReminderJob.set(wait_until: send_at).perform_later(pref.household_id, pref.person_id, :scheduled, time)
    end
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
