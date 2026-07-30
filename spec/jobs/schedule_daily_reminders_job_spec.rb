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
    PersonAccessGrant.where(household: household).delete_all
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

  it 'records a past occurrence instead of enqueuing it' do
    events = []
    allow(Observability::CanonicalLogger).to receive(:write) { |event| events << event.to_h }
    create(:notification_preference, person: person, morning_time: '05:00:00', afternoon_time: nil,
                                     evening_time: nil, night_time: nil)

    expect { described_class.perform_now }.not_to have_enqueued_job(MedicationReminderJob)
    expect(events).to include(
      include(
        'event.name' => 'notification.stage',
        'medtracker.reason' => 'past_occurrence',
        'medtracker.workflow.stage' => 'reminder_enqueue'
      )
    )
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
    preference = create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                                  evening_time: nil, night_time: nil, dose_due_enabled: false,
                                                  missed_dose_enabled: true)
    job = described_class.new
    allow(job).to receive(:configured_times_for).and_return(['07:15'])

    expect do
      job.send(:enqueue_reminders_for, preference)
    end.to have_enqueued_job(MissedDoseNotificationJob)
      .with(household.id, person.id, '2026-05-12', '07:15')
      .at(Time.zone.local(2026, 5, 12, 7, 45))
  end

  it 'propagates one opaque workflow from enqueue into the missed-dose job' do
    events = []
    allow(Observability::CanonicalLogger).to receive(:write) { |event| events << event.to_h }
    preference = create(:notification_preference, person: person, morning_time: nil, afternoon_time: nil,
                                                  evening_time: nil, night_time: nil, dose_due_enabled: false,
                                                  missed_dose_enabled: true)
    job = described_class.new
    allow(job).to receive(:configured_times_for).and_return(['07:15'])

    job.send(:enqueue_reminders_for, preference)

    job_data = enqueued_jobs.find { |entry| entry.fetch(:job) == MissedDoseNotificationJob }
    propagation = job_data.fetch('medtracker_observability_context')
    enqueue_event = reminder_enqueue_event(events)

    expect(enqueue_event['medtracker.workflow.id']).to eq(propagation.fetch('workflow.id'))
    expect(enqueue_event['medtracker.attempt.id']).to eq(propagation.fetch('attempt.id'))
    expect(propagation.keys).to contain_exactly('workflow.id', 'causation.id', 'attempt.id', 'expires_at')
  end

  it 'enqueues missed-dose checks for every child managed by an enabled recipient' do
    parent = people(:jane)
    children = [people(:child_patient), people(:child_user_person)]
    enable_missed_dose_notifications(parent)
    children.each { |child| prepare_managed_schedule(manager: parent, target: child, relationship_type: :parent) }

    expect(ManagedMissedDoseNotificationSubjectsQuery.new(household: household).call)
      .to match_array(children)
    expect_missed_dose_checks_for(children)
  end

  it 'only schedules missed-dose checks for a managed adult after opt in' do
    manager = people(:jane)
    managed_adult = people(:bob)
    enable_missed_dose_notifications(manager)
    grant = prepare_managed_schedule(manager: manager, target: managed_adult, relationship_type: :family_member)
    expect_no_missed_dose_check(managed_adult)

    clear_enqueued_jobs
    grant.update!(missed_dose_notifications_enabled: true)
    expect(ManagedMissedDoseNotificationSubjectsQuery.new(household: household).call)
      .to include(managed_adult)
    expect_missed_dose_check(managed_adult)
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

  def enable_missed_dose_notifications(target)
    create(:notification_preference, person: target, dose_due_enabled: false, missed_dose_enabled: true)
  end

  def reminder_enqueue_event(events)
    events.find do |event|
      event['event.name'] == 'notification.stage' && event['medtracker.reason'] == 'enqueued'
    end
  end

  def prepare_managed_schedule(manager:, target:, relationship_type:)
    clear_medication_activity(target)
    grant = grant_management_access(manager: manager, target: target, relationship_type: relationship_type)
    create_daily_schedule(target)
    grant
  end

  def create_daily_schedule(target)
    create(:schedule, person: target, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] },
                      start_date: Date.new(2026, 5, 11), end_date: Date.new(2026, 6, 12))
  end

  def expect_missed_dose_checks_for(children)
    expect do
      described_class.perform_now
    end.to have_enqueued_job(MissedDoseNotificationJob)
      .with(household.id, children.first.id, '2026-05-12', '07:15')
      .and(
        have_enqueued_job(MissedDoseNotificationJob)
          .with(household.id, children.second.id, '2026-05-12', '07:15')
      )
  end

  def expect_no_missed_dose_check(target)
    expect do
      described_class.perform_now
    end.not_to have_enqueued_job(MissedDoseNotificationJob)
      .with(household.id, target.id, '2026-05-12', '07:15')
  end

  def expect_missed_dose_check(target)
    expect do
      described_class.perform_now
    end.to have_enqueued_job(MissedDoseNotificationJob)
      .with(household.id, target.id, '2026-05-12', '07:15')
  end

  def grant_management_access(manager:, target:, relationship_type:)
    membership = household.household_memberships.find_or_create_by!(account: manager.account) do |record|
      record.person = manager
      record.role = :member
      record.status = :active
    end
    PersonAccessGrant.create!(
      household: household,
      household_membership: membership,
      person: target,
      access_level: :manage,
      relationship_type: relationship_type,
      granted_by_membership: membership
    )
  end

  def clear_medication_activity(target)
    MedicationTake.where(schedule_id: target.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: target.person_medications.select(:id)).delete_all
    target.schedules.destroy_all
    target.person_medications.destroy_all
  end
end
