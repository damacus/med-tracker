# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MissedDoseNotificationJob do
  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }
  let(:household) { person.household }
  let(:scheduled_on) { '2026-05-12' }
  let(:scheduled_time) { '07:15' }

  before do
    MedicationTake.where(schedule_id: person.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: person.person_medications.select(:id)).delete_all
    person.schedules.destroy_all
    person.person_medications.destroy_all
    PushSubscription.create!(
      account: person.account,
      endpoint: 'https://fcm.googleapis.com/fcm/send/missed-dose',
      p256dh: 'public-key',
      auth: 'auth-secret'
    )
    person.create_notification_preference!(enabled: true, missed_dose_enabled: true)
    allow(PushNotificationService).to receive(:send_to_account)
  end

  it 'sends one private notification when a scheduled dose is overdue' do
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Medication reminder',
      body: 'A dose may have been missed.',
      path: "/households/#{household.slug}/dashboard"
    )
    expect(NotificationEvent.where(event_type: 'missed_dose').count).to eq(1)
  end

  it 'suppresses duplicates for the same scheduled occurrence' do
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      2.times { described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time) }
    end

    expect(PushNotificationService).to have_received(:send_to_account).once
  end

  it 'does not send when the dose was taken in the dose window' do
    schedule = create_schedule
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.local(2026, 5, 12, 7, 20))

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'does not send when missed-dose notifications are disabled' do
    person.notification_preference.update!(missed_dose_enabled: false)
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'records a skip when the account has no active push subscriptions' do
    person.account.push_subscriptions.destroy_all
    create_schedule

    travel_to Time.zone.local(2026, 5, 12, 7, 46) do
      described_class.perform_now(household.id, person.id, scheduled_on, scheduled_time)
    end

    event = NotificationEvent.find_by!(event_type: 'missed_dose')
    expect(event.skipped_reason).to eq('no_active_push_subscriptions')
    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  def create_schedule
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily,
                      schedule_config: { 'times' => [scheduled_time] },
                      start_date: Date.parse(scheduled_on) - 1.day, end_date: Date.parse(scheduled_on) + 1.month)
  end
end
