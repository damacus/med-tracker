# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe MedicationReminderJob do
  self.use_transactional_tests = false

  fixtures :medications, :dosages

  let(:household) { medications(:vitamin_d).household }
  let(:account) do
    Account.create!(email: "reminder-concurrency-#{SecureRandom.hex(4)}@example.test", status: :verified)
  end
  let(:person) do
    Person.create!(
      household: household,
      account: account,
      name: 'Reminder Concurrency Person',
      date_of_birth: 30.years.ago,
      person_type: :adult,
      has_capacity: true
    )
  end
  let(:deliveries) { Queue.new }
  let!(:sync_record_ids_before) do
    { change_events: ApiChangeEvent.ids, tombstones: ApiTombstone.ids }
  end

  before do
    create_concurrency_records
    allow(PushNotificationService).to receive(:send_to_account) { deliveries << true }
  end

  after { cleanup_concurrency_records }

  it 'sends one due notification when the same occurrence executes concurrently' do
    perform_concurrently

    expect(deliveries.size).to eq(1)
    expect(NotificationEvent.where(event_type: 'dose_due', person: person).count).to eq(1)
  end

  def create_concurrency_records
    TenantContext.with(account: nil, household: household) do
      person.create_notification_preference!(enabled: true, dose_due_enabled: true)
      create(:schedule, **concurrency_schedule_attributes)
    end
  end

  def concurrency_schedule_attributes
    {
      person: person,
      medication: medications(:vitamin_d),
      dosage: dosages(:vitamin_d_daily),
      frequency: 'Once daily',
      schedule_type: :daily,
      schedule_config: { 'times' => ['07:15'] },
      start_date: Date.current - 1.day,
      end_date: Date.current + 1.month
    }
  end

  def perform_concurrently
    ready = Queue.new
    start = Queue.new
    threads = 2.times.map { reminder_thread(ready, start) }
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    threads.each { join_thread(it) }
  ensure
    Array(threads).each(&:kill)
  end

  def reminder_thread(ready, start)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        Time.use_zone('Europe/London') do
          ready << true
          Timeout.timeout(10) { start.pop }
          described_class.perform_now(household.id, person.id, :scheduled, '07:15')
        end
      end
    end
  end

  def join_thread(thread)
    return thread.value if thread.join(10)

    raise Timeout::Error, 'timed out waiting for reminder worker'
  end

  def cleanup_concurrency_records
    TenantContext.with(account: nil, household: household) do
      NotificationEvent.where(person: person).delete_all
      person.destroy!
    end
    account.destroy!
    cleanup_sync_records
  end

  def cleanup_sync_records
    ApiChangeEvent.where.not(id: sync_record_ids_before.fetch(:change_events)).delete_all
    ApiTombstone.where.not(id: sync_record_ids_before.fetch(:tombstones)).delete_all
  end
end
