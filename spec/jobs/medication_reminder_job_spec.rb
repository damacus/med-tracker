# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReminderJob do
  include ActiveJob::TestHelper

  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }
  let(:household) { person.household }

  before do
    MedicationTake.where(schedule_id: person.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: person.person_medications.select(:id)).delete_all
    person.schedules.destroy_all
    person.person_medications.destroy_all
    person.create_notification_preference!(enabled: true, dose_due_enabled: true, private_text_enabled: false)
    allow(PushNotificationService).to receive(:send_to_account)
  end

  it 'sends scheduled-time reminders only for active schedules configured at that time' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['19:45'] })

    described_class.perform_now(household.id, person.id, :scheduled, '07:15')

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Medication Reminder',
      body: '07:15 medications: Vitamin D',
      path: "/households/#{household.slug}/dashboard",
      notification_kind: :dose_due
    )
  end

  it 'sends one due notification when the same occurrence is replayed sequentially' do
    create_vitamin_schedule(
      time: '07:15', start_date: Date.new(2026, 5, 11), end_date: Date.new(2026, 6, 12)
    )

    Time.use_zone('Europe/London') do
      travel_to Time.zone.local(2026, 5, 12, 7, 15) do
        2.times { described_class.perform_now(household.id, person.id, :scheduled, '07:15') }
      end
    end

    expect(PushNotificationService).to have_received(:send_to_account).once
    expect(NotificationEvent.where(event_type: 'dose_due').count).to eq(1)
  end

  it 'canonicalizes equivalent serialized three- and four-argument jobs' do
    person.notification_preference.update!(morning_time: '07:15:00')
    create_vitamin_schedule(
      time: '07:15', start_date: Date.new(2026, 5, 11), end_date: Date.new(2026, 6, 12)
    )

    Time.use_zone('Europe/London') do
      intended_at = Time.zone.local(2026, 5, 12, 7, 15)
      travel_to(intended_at) { perform_serialized_legacy_jobs(intended_at) }
    end

    event = NotificationEvent.find_by!(event_type: 'dose_due')
    expect(PushNotificationService).to have_received(:send_to_account).once
    expect(event.event_key).to eq("dose-due:#{person.id}:2026-05-12T06:15:00Z")
    expect(event.metadata).to include('delivery_status' => 'delivery_unknown')
  end

  it 'normalizes explicit London occurrences across summer and winter offsets' do
    create_vitamin_schedule(
      time: '07:15', start_date: Date.new(2026, 1, 1), end_date: Date.new(2026, 12, 31)
    )

    Time.use_zone('Europe/London') do
      [Time.zone.local(2026, 5, 12, 7, 15), Time.zone.local(2026, 12, 12, 7, 15)].each do |intended_at|
        travel_to intended_at do
          described_class.perform_now(household.id, person.id, :scheduled, '07:15', intended_at)
        end
      end
    end

    expect(NotificationEvent.where(event_type: 'dose_due').order(:event_key).pluck(:event_key)).to contain_exactly(
      "dose-due:#{person.id}:2026-05-12T06:15:00Z",
      "dose-due:#{person.id}:2026-12-12T07:15:00Z"
    )
    expect(PushNotificationService).to have_received(:send_to_account).twice
  end

  it 'keeps an unknown delivery reserved when the application delivery raises' do
    create_vitamin_schedule(
      time: '07:15', start_date: Date.new(2026, 5, 11), end_date: Date.new(2026, 6, 12)
    )
    allow(PushNotificationService).to receive(:send_to_account).and_raise(IOError, 'provider result unknown')

    Time.use_zone('Europe/London') do
      intended_at = Time.zone.local(2026, 5, 12, 7, 15)
      travel_to(intended_at) { perform_unknown_delivery_replay(intended_at) }
    end

    event = NotificationEvent.find_by!(event_type: 'dose_due')
    expect(PushNotificationService).to have_received(:send_to_account).once
    expect(event).to have_attributes(sent_at: nil)
    expect(event.metadata).to include('delivery_status' => 'delivery_unknown')
  end

  it 'does not send scheduled-time reminders for doses already taken today' do
    schedule = create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                                 frequency: 'Once daily', schedule_type: :daily,
                                 schedule_config: { 'times' => ['07:15'] })
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.noon)

    described_class.perform_now(household.id, person.id, :scheduled, '07:15')

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send later scheduled-time reminders after the medication was taken today' do
    schedule = create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                                 frequency: 'Twice daily', schedule_type: :multiple_daily,
                                 schedule_config: { 'times' => %w[07:15 19:45] })
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.beginning_of_day + 8.hours)

    travel_to Time.zone.today.beginning_of_day + 19.hours + 45.minutes do
      described_class.perform_now(household.id, person.id, :scheduled, '19:45')
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send scheduled-time reminders when the medication was taken today through a direct assignment' do
    create_vitamin_schedule(time: '19:45')
    person_medication = create_routine_vitamin
    take_person_medication(person_medication)

    travel_to Time.zone.today.beginning_of_day + 19.hours + 45.minutes do
      described_class.perform_now(household.id, person.id, :scheduled, '19:45')
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send scheduled-time reminders for as-needed schedules' do
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :prn, frequency: 'As needed', schedule_config: { 'times' => ['07:15'] })

    described_class.perform_now(household.id, person.id, :scheduled, '07:15')

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send scheduled-time reminders for paused schedules' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      active: false, frequency: 'Once daily', schedule_type: :daily,
                      schedule_config: { 'times' => ['07:15'] })

    described_class.perform_now(household.id, person.id, :scheduled, '07:15')

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send period reminders for schedules without configured times' do
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      frequency: 'Every 6-8 hours', schedule_type: :daily, schedule_config: {},
                      max_daily_doses: 3, min_hours_between_doses: 6)

    described_class.perform_now(household.id, person.id, :afternoon)

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'sends period reminders only for due routine medications' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :prn, frequency: 'As needed', schedule_config: { 'times' => ['19:45'] })
    create(:person_medication, :as_needed, person: person, medication: medications(:paracetamol),
                                           dosage: dosages(:paracetamol_adult))

    described_class.perform_now(household.id, person.id, :morning)

    expect(PushNotificationService).to have_received(:send_to_account) do |_account, payload|
      expect(payload[:body]).to eq('Morning medications: Vitamin D')
      expect(payload[:body]).not_to include('Ibuprofen')
      expect(payload[:body]).not_to include('Paracetamol')
    end
  end

  it 'does not send period reminders after a medication was taken today even when more doses are allowed' do
    schedule = create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                                 frequency: 'Every 6 hours', schedule_type: :multiple_daily,
                                 schedule_config: { 'times' => %w[08:00 14:00 20:00] },
                                 max_daily_doses: 3, min_hours_between_doses: 6)
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.beginning_of_day + 8.hours)

    travel_to Time.zone.today.beginning_of_day + 14.hours do
      described_class.perform_now(household.id, person.id, :afternoon)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send period reminders when a direct medication was taken today through a schedule' do
    schedule = create_vitamin_schedule(time: '07:15')
    create_routine_vitamin
    take_schedule(schedule)

    travel_to Time.zone.today.beginning_of_day + 14.hours do
      described_class.perform_now(household.id, person.id, :afternoon)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send period reminders when routine medications have already been taken today' do
    schedule = create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                                 frequency: 'Once daily', schedule_type: :daily,
                                 schedule_config: { 'times' => ['07:15'] },
                                 max_daily_doses: 1)
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.noon)

    described_class.perform_now(household.id, person.id, :morning)

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send period reminders for paused routine medications' do
    create(:person_medication, :routine, person: person, medication: medications(:vitamin_d),
                                         dosage: dosages(:vitamin_d_daily), active: false)

    described_class.perform_now(household.id, person.id, :morning)

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'sends private text when private_text_enabled is true' do
    person.notification_preference.update!(private_text_enabled: true)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })

    described_class.perform_now(household.id, person.id, :scheduled, '07:15')

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Medication reminder',
      body: 'A dose is due.',
      path: "/households/#{household.slug}/dashboard",
      notification_kind: :dose_due
    )
  end

  it 'does not send anything when dose_due_enabled is false' do
    person.notification_preference.update!(dose_due_enabled: false)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })

    described_class.perform_now(household.id, person.id, :scheduled, '07:15')

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  context 'when the household lifecycle is unavailable' do
    around do |example|
      original_queue_adapter = ActiveJob::Base.queue_adapter
      ActiveJob::Base.queue_adapter = :test
      example.run
    ensure
      clear_enqueued_jobs
      ActiveJob::Base.queue_adapter = original_queue_adapter
    end

    before do
      create(:schedule, household: household, person: person, medication: medications(:vitamin_d),
                        dosage: dosages(:vitamin_d_daily), schedule_type: :daily,
                        schedule_config: { 'times' => ['07:15'] })
    end

    it 'does not deliver a queued reminder' do
      allow(TenantContext).to receive(:with).and_call_original

      %i[held offboarded purged].each do |state|
        household.update!(lifecycle_state: state)
        described_class.perform_now(household.id, person.id, :scheduled, '07:15')
      end

      expect(TenantContext).not_to have_received(:with)
      expect(PushNotificationService).not_to have_received(:send_to_account)
    end

    it 'does not enqueue new reminders' do
      %i[held offboarded purged].each do |state|
        household.update!(lifecycle_state: state)
        expect { ScheduleDailyRemindersJob.perform_now }.not_to have_enqueued_job(described_class)
      end
    end
  end

  def create_vitamin_schedule(time:, start_date: Time.zone.today, end_date: 1.year.from_now.to_date)
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily,
                      schedule_config: { 'times' => [time] }, start_date:, end_date:)
  end

  def perform_serialized_legacy_jobs(intended_at)
    [legacy_period_job, legacy_scheduled_job].each { |job| perform_serialized(job, intended_at) }
  end

  def legacy_period_job
    described_class.new(household.id, person.id, :morning)
  end

  def legacy_scheduled_job
    described_class.new(household.id, person.id, :scheduled, '07:15')
  end

  def perform_serialized(job, intended_at)
    job.scheduled_at = intended_at
    ActiveJob::Base.deserialize(job.serialize).perform_now
  end

  def perform_unknown_delivery_replay(intended_at)
    expect { perform_explicit_occurrence(intended_at) }.to raise_error(IOError, 'provider result unknown')
    expect { perform_explicit_occurrence(intended_at) }.not_to raise_error
  end

  def perform_explicit_occurrence(intended_at)
    described_class.perform_now(household.id, person.id, :scheduled, '07:15', intended_at)
  end

  def create_routine_vitamin
    create(
      :person_medication,
      :routine,
      person: person,
      medication: medications(:vitamin_d),
      dosage: dosages(:vitamin_d_daily)
    )
  end

  def take_person_medication(person_medication)
    create(
      :medication_take,
      :for_person_medication,
      person_medication: person_medication,
      taken_at: Time.zone.today.beginning_of_day + 8.hours
    )
  end

  def take_schedule(schedule)
    create(
      :medication_take,
      :for_schedule,
      schedule: schedule,
      taken_at: Time.zone.today.beginning_of_day + 8.hours
    )
  end
end
