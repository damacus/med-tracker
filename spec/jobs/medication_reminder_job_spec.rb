# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReminderJob do
  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }

  before do
    allow(PushNotificationService).to receive(:send_to_account)
  end

  it 'sends scheduled-time reminders only for active schedules configured at that time' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :daily, schedule_config: { 'times' => ['19:45'] })

    described_class.perform_now(person.id, :scheduled, '07:15')

    expect(PushNotificationService).to have_received(:send_to_account).with(
      person.account,
      title: 'Medication Reminder',
      body: '07:15 medications: Vitamin D',
      path: '/'
    )
  end

  it 'preserves period reminders for all active medications' do
    create(:schedule, person: person, medication: medications(:vitamin_d), dosage: dosages(:vitamin_d_daily),
                      schedule_type: :daily, schedule_config: { 'times' => ['07:15'] })
    create(:schedule, person: person, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_adult),
                      schedule_type: :daily, schedule_config: { 'times' => ['19:45'] })

    described_class.perform_now(person.id, :morning)

    expect(PushNotificationService).to have_received(:send_to_account) do |_account, payload|
      expect(payload[:body]).to include('Morning medications:')
      expect(payload[:body]).to include('Vitamin D')
      expect(payload[:body]).to include('Ibuprofen')
    end
  end
end
