# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LowStockNotificationJob do
  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }
  let(:household) { person.household }
  let(:medication) { medications(:vitamin_d) }
  let(:take_id) { 123 }

  before do
    MedicationTake.where(schedule_id: person.schedules.select(:id)).delete_all
    MedicationTake.where(person_medication_id: person.person_medications.select(:id)).delete_all
    person.schedules.destroy_all
    person.person_medications.destroy_all
    medication.update!(household: household, location: locations(:home))
    create(:schedule, person: person, medication: medication, dosage: dosages(:vitamin_d_daily),
                      frequency: 'Once daily', schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    PushSubscription.create!(
      account: person.account,
      endpoint: 'https://fcm.googleapis.com/fcm/send/low-stock',
      p256dh: 'public-key',
      auth: 'auth-secret'
    )
    person.create_notification_preference!(enabled: true, low_stock_enabled: true)
    allow(PushNotificationService).to receive(:send_to_account)
  end

  it 'sends one private notification to an eligible person assigned the medication' do
    described_class.perform_now(household.id, medication.id, take_id)

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Stock reminder',
      body: 'A medication may be running low.',
      path: "/households/#{household.slug}/dashboard"
    )
    expect(NotificationEvent.where(event_type: 'low_stock').count).to eq(1)
  end

  it 'suppresses duplicates for the same unchanged low-stock event' do
    2.times { described_class.perform_now(household.id, medication.id, take_id) }

    expect(PushNotificationService).to have_received(:send_to_account).once
  end

  it 'allows a later notification for a later threshold crossing' do
    described_class.perform_now(household.id, medication.id, take_id)
    described_class.perform_now(household.id, medication.id, take_id + 1)

    expect(PushNotificationService).to have_received(:send_to_account).twice
  end

  it 'does not send when low-stock notifications are disabled' do
    person.notification_preference.update!(low_stock_enabled: false)

    described_class.perform_now(household.id, medication.id, take_id)

    expect(PushNotificationService).not_to have_received(:send_to_account)
  end

  it 'records a skip when the account has no active push subscriptions' do
    person.account.push_subscriptions.destroy_all

    described_class.perform_now(household.id, medication.id, take_id)

    event = NotificationEvent.find_by!(event_type: 'low_stock')
    expect(event.skipped_reason).to eq('no_active_push_subscriptions')
    expect(PushNotificationService).not_to have_received(:send_to_account)
  end
end
