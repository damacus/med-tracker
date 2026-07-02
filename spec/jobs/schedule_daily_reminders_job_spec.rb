# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ScheduleDailyRemindersJob do
  include ActiveJob::TestHelper

  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }
  let(:household) { person.household }

  around do |example|
    original_queue_adapter = ActiveJob::Base.queue_adapter
    ActiveJob::Base.queue_adapter = :test

    example.run
  ensure
    clear_enqueued_jobs
    ActiveJob::Base.queue_adapter = original_queue_adapter
  end

  before do
    travel_to Time.zone.local(2026, 5, 12, 6, 0)
  end

  def count_schedule_queries(&)
    count = 0

    subscriber = lambda do |_name, _start, _finish, _id, payload|
      sql = payload[:sql]
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      count += 1 if sql.include?('FROM "schedules"') && sql.include?('"schedules"."person_id"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end

  it 'preserves period reminders from enabled notification preferences' do
    create(:notification_preference, person: person, morning_time: '08:00:00', afternoon_time: nil,
                                     evening_time: nil, night_time: nil)

    expect do
      described_class.perform_now
    end.to have_enqueued_job(MedicationReminderJob)
      .with(household.id, person.id, :morning)
      .at(Time.zone.local(2026, 5, 12, 8, 0))
  end

  it 'enqueues exact reminders for active schedule times when dose-due preferences are enabled' do
    create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil, dose_due_enabled: true,
                                     missed_dose_enabled: false)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Twice daily', schedule_type: :multiple_daily,
                      schedule_config: { 'times' => ['07:15'] })

    expect do
      described_class.perform_now
    end.to have_enqueued_job(MedicationReminderJob)
      .with(household.id, person.id, :scheduled, '07:15')
      .at(Time.zone.local(2026, 5, 12, 7, 15))
  end

  it 'enqueues missed-dose checks after active schedule times when missed-dose preferences are enabled' do
    create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil, dose_due_enabled: false,
                                     missed_dose_enabled: true)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :multiple_daily,
                      schedule_config: { 'times' => ['07:15'] })

    expect do
      described_class.perform_now
    end.to have_enqueued_job(MissedDoseNotificationJob)
      .with(household.id, person.id, '2026-05-12', '07:15')
      .at(Time.zone.local(2026, 5, 12, 7, 45))
  end

  it 'does not enqueue exact reminders for as-needed schedules' do
    create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil)
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :prn, frequency: 'As needed', schedule_config: { 'times' => ['07:15'] })

    expect do
      described_class.perform_now
    end.not_to have_enqueued_job(MedicationReminderJob)
      .with(household.id, person.id, :scheduled, '07:15')
  end

  it 'does not enqueue exact reminders for schedules that do not apply today' do
    create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once weekly', schedule_type: :weekly,
                      schedule_config: { 'weekdays' => ['wednesday'], 'times' => ['07:15'] })

    expect do
      described_class.perform_now
    end.not_to have_enqueued_job(MedicationReminderJob)
      .with(household.id, person.id, :scheduled, '07:15')
  end

  it 'does not enqueue exact reminders for paused schedules' do
    create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      active: false, frequency: 'Once daily', schedule_type: :daily,
                      schedule_config: { 'times' => ['07:15'] })

    expect do
      described_class.perform_now
    end.not_to have_enqueued_job(MedicationReminderJob)
      .with(household.id, person.id, :scheduled, '07:15')
  end

  it 'loads schedule times once for enabled notification preferences' do
    create(:notification_preference, person: people(:john), morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil)
    create(:notification_preference, person: people(:jane), morning_time: nil, afternoon_time: nil,
                                     evening_time: nil, night_time: nil)
    create(:schedule, person: people(:john), medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: people(:jane), medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      schedule_type: :daily, schedule_config: { 'times' => ['07:45'] })

    expect(count_schedule_queries { described_class.perform_now }).to eq(1)
  end

  it 'does not enqueue schedule-time reminders when notification preferences are disabled' do
    create(:notification_preference, person: person, enabled: false)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })

    expect do
      described_class.perform_now
    end.not_to have_enqueued_job(MedicationReminderJob)
  end
end
