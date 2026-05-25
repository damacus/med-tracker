# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReminderJob do
  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }

  before do
    person.schedules.destroy_all
    person.person_medications.destroy_all
    allow(PushNotificationService).to receive(:send_to_account)
  end

  it 'sends scheduled-time reminders only for active schedules configured at that time' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['19:45'] })

    described_class.perform_now(person.id, :scheduled, '07:15')

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Medication Reminder',
      body: '07:15 medications: Vitamin D',
      path: '/'
    )
  end

  it 'does not send scheduled-time reminders for doses already taken today' do
    schedule = create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                                 frequency: 'Once daily', schedule_type: :daily,
                                 schedule_config: { 'times' => ['07:15'] })
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.noon)

    described_class.perform_now(person.id, :scheduled, '07:15')

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send later scheduled-time reminders after the medication was taken today' do
    schedule = create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                                 frequency: 'Twice daily', schedule_type: :multiple_daily,
                                 schedule_config: { 'times' => %w[07:15 19:45] })
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.beginning_of_day + 8.hours)

    travel_to Time.zone.today.beginning_of_day + 19.hours + 45.minutes do
      described_class.perform_now(person.id, :scheduled, '19:45')
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send scheduled-time reminders for as-needed schedules' do
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :prn, frequency: 'As needed', schedule_config: { 'times' => ['07:15'] })

    described_class.perform_now(person.id, :scheduled, '07:15')

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send period reminders for schedules without configured times' do
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      frequency: 'Every 6-8 hours', schedule_type: :daily, schedule_config: {},
                      max_daily_doses: 3, min_hours_between_doses: 6)

    described_class.perform_now(person.id, :afternoon)

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'sends period reminders only for due routine medications' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :prn, frequency: 'As needed', schedule_config: { 'times' => ['19:45'] })
    create(:person_medication, :as_needed, person: person, medication: medications(:paracetamol),
                                           dosage: dosages(:paracetamol_adult))

    described_class.perform_now(person.id, :morning)

    expect(PushNotificationService).to have_received(:send_to_account) do |_account, payload|
      expect(payload[:body]).to include('Morning medications:')
      expect(payload[:body]).to include('Vitamin D')
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
      described_class.perform_now(person.id, :afternoon)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send period reminders when routine medications have already been taken today' do
    schedule = create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                                 frequency: 'Once daily', schedule_type: :daily,
                                 schedule_config: { 'times' => ['07:15'] },
                                 max_daily_doses: 1)
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.today.noon)

    described_class.perform_now(person.id, :morning)

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end
end
