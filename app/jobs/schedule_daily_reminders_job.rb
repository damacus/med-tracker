# frozen_string_literal: true

class ScheduleDailyRemindersJob < ApplicationJob
  queue_as :default

  PERIODS = NotificationPreference::PERIODS

  def perform
    NotificationPreference.where(enabled: true).includes(person: :account).find_each do |pref|
      next unless pref.person&.account

      enqueue_reminders_for(pref)
    end
  end

  private

  def enqueue_reminders_for(pref)
    PERIODS.each do |period|
      time = pref.time_for_period(period)
      next unless time

      send_at = build_send_time(time)
      next if send_at < Time.current

      MedicationReminderJob.set(wait_until: send_at).perform_later(pref.person_id, period)
    end
  end

  def build_send_time(time)
    today = Time.zone.today
    Time.zone.local(today.year, today.month, today.day, time.hour, time.min)
  end
end
