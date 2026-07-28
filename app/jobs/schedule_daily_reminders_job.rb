# frozen_string_literal: true

class ScheduleDailyRemindersJob < ApplicationJob
  queue_as :default

  PERIODS = NotificationPreference::PERIODS

  class BulkEnqueueError < StandardError
    def initialize(jobs)
      details = jobs.map do |job|
        "#{job.class.name}: #{job.enqueue_error&.message || 'unknown enqueue error'}"
      end
      super("Failed to enqueue #{jobs.size} reminder job(s): #{details.join(', ')}")
    end
  end

  def perform
    jobs = []

    Household.operational.find_each do |household|
      TenantContext.with(account: nil, household: household) do
        jobs.concat(build_household_reminders(household))
      end
    end

    bulk_enqueue(jobs)
  end

  private

  def build_household_reminders(household)
    jobs = []

    NotificationPreference.where(household: household, enabled: true)
                          .where('dose_due_enabled = ? OR missed_dose_enabled = ?', true, true)
                          .includes(person: [:account, { schedules: %i[medication medication_takes] }])
                          .find_each do |pref|
      next unless pref.person&.account

      jobs.concat(build_reminders_for(pref))
    rescue StandardError => e
      Rails.logger.error("Failed to schedule reminders for preference #{pref.id}: #{e.class}: #{e.message}")
    end

    jobs
  end

  def build_reminders_for(pref)
    period_reminders = pref.dose_due_enabled ? build_period_reminders_for(pref) : []
    period_reminders + build_schedule_time_reminders_for(pref)
  end

  def build_period_reminders_for(pref)
    PERIODS.filter_map do |period|
      time = pref.time_for_period(period)
      next unless time

      send_at = build_send_time(time)
      next if send_at < Time.current

      MedicationReminderJob.new(pref.household_id, pref.person_id, period).set(wait_until: send_at)
    end
  end

  def build_schedule_time_reminders_for(pref)
    configured_times_for(pref).each_with_object([]) do |time, jobs|
      send_at = build_send_time_from_configured_time(time)
      next if send_at.blank? || send_at < Time.current

      if pref.dose_due_enabled
        jobs << MedicationReminderJob
                .new(pref.household_id, pref.person_id, :scheduled, time)
                .set(wait_until: send_at)
      end
      jobs << build_missed_dose_check_for(pref, send_at, time) if pref.missed_dose_enabled
    end
  end

  def build_missed_dose_check_for(pref, send_at, time)
    MissedDoseNotificationJob
      .new(pref.household_id, pref.person_id, send_at.to_date.iso8601, time)
      .set(wait_until: send_at + MissedDoseNotificationJob::GRACE_PERIOD)
  end

  def bulk_enqueue(jobs)
    return if jobs.empty?

    ActiveJob.perform_all_later(jobs)
    failed_jobs = jobs.reject(&:successfully_enqueued?)
    raise BulkEnqueueError, failed_jobs if failed_jobs.any?
  end

  def configured_times_for(pref)
    MedicationReminderEligibilityQuery.new(person: pref.person).configured_times
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
