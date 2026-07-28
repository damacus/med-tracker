# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe 'API v1 medication-take sync concurrency' do
  self.use_transactional_tests = false

  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:record_state) { { loaded: false } }
  let(:records) do
    PaperTrail.request(enabled: false) { concurrency_records }.tap { record_state[:loaded] = true }
  end
  let!(:version_ids_before) { AuditLedgerEntry.where(source_table: 'versions').pluck(:source_id) }
  let!(:change_event_ids_before) { ApiChangeEvent.ids }

  before do
    version_ledger_trigger(:disable)
    allow(Audit::Event).to receive(:record!)
    allow(AuthTokenAuditLogger).to receive(:new)
      .and_return(instance_double(AuthTokenAuditLogger, record: nil))
    records
  end

  after do
    cleanup_records(records) if record_state[:loaded]
  ensure
    version_ledger_trigger(:enable)
  end

  def cleanup_records(created_records)
    take_ids = MedicationTake.where(client_uuid: created_records.fetch(:client_uuid)).pluck(:id)
    cleanup_domain_records(take_ids)
    cleanup_audit_records
  end

  def cleanup_audit_records
    ApiChangeEvent.where.not(id: change_event_ids_before).delete_all
    PaperTrail::Version.where.not(id: version_ids_before).delete_all
  end

  def cleanup_domain_records(take_ids)
    MedicationTake.where(id: take_ids).delete_all
    Schedule.where(id: schedule.id).delete_all
    PaperTrail.request(enabled: false) { medication.reload.destroy! }
    ApiSession.where(id: session_id).delete_all
  end

  def version_ledger_trigger(action)
    ActiveRecord::Base.connection.execute(
      "ALTER TABLE versions #{action.to_s.upcase} TRIGGER append_versions_to_audit_ledger"
    )
  end

  it 'converges concurrent batch retries on one take and one set of side effects' do
    ready = Queue.new
    release = Queue.new
    gate = Mutex.new
    counter = { calls: 0 }
    pause_initial_dose_calls(ready, release, gate, counter)
    initial_supply = medication.current_supply

    sessions = Array.new(2) { ActionDispatch::Integration::Session.new(Rails.application) }
    workers = sessions.map { |session| start_request(session) }

    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { release << true }
    responses = workers.map { |worker| join_worker(worker) }

    expect(responses.map { |result| result.fetch(:status) }).to eq([201, 201])
    results = responses.map { |result| result.dig(:body, 'data', 'results', 0) }
    expect(results.map { |result| result.fetch('record_portable_id') }.uniq.one?).to be(true)
    expect(results.map { |result| result.fetch('replayed') }).to contain_exactly(false, true)
    take = MedicationTake.find_by!(client_uuid: client_uuid)
    expect(MedicationTake.where(client_uuid: client_uuid).count).to eq(1)
    expect(medication.reload.current_supply).to be < initial_supply
    expect(PaperTrail::Version.where(item_type: 'MedicationTake', item_id: take.id).count).to eq(1)
    expect(ApiChangeEvent.where(record_type: 'MedicationTake', record_id: take.id).count).to eq(1)
  end

  def pause_initial_dose_calls(ready, release, gate, counter)
    allow(MedicationAdministration::RecordDose).to receive(:new).and_wrap_original do |original, *arguments|
      pause_dose_service(original.call(*arguments), ready, release, gate, counter)
    end
  end

  def pause_dose_service(service, ready, release, gate, counter)
    original_call = service.method(:call)
    allow(service).to receive(:call) do |**keywords|
      pause_dose_call(ready, release) if initial_dose_call?(gate, counter)
      original_call.call(**keywords)
    end
    service
  end

  def initial_dose_call?(gate, counter)
    gate.synchronize do
      counter[:calls] += 1
      counter[:calls] <= 2
    end
  end

  def pause_dose_call(ready, release)
    ready << true
    Timeout.timeout(10) { release.pop }
  end

  def concurrency_records
    user = users(:admin)
    household = ensure_api_household_for(user)
    medication = concurrency_medication(household, locations(:home))
    schedule = concurrency_schedule(household, medication)
    session, access_token = concurrency_api_session(user, household)
    concurrency_record_attributes(household, medication, schedule, session, access_token)
  end

  def concurrency_api_session(user, household)
    membership = user.person.account.household_memberships.find_by!(household: household)
    ApiSession.issue_for(
      account: user.person.account,
      household_membership: membership,
      device_name: 'RSpec concurrency client'
    )
  end

  def concurrency_record_attributes(household, medication, schedule, session, access_token)
    {
      household_id: household.id,
      headers: api_auth_headers(access_token),
      session_id: session.id,
      medication: medication,
      schedule: schedule,
      client_uuid: SecureRandom.uuid
    }
  end

  def concurrency_medication(household, location)
    Medication.create!(
      name: "Concurrency Medication #{SecureRandom.hex(4)}",
      household: household,
      location: location,
      dose_amount: 500,
      dose_unit: 'mg',
      current_supply: 10,
      supply_at_last_restock: 10,
      expiry_date: 1.year.from_now
    )
  end

  def concurrency_schedule(household, medication)
    Schedule.create!(
      household: household,
      person: people(:jane),
      medication: medication,
      dose_amount: 500,
      dose_unit: 'mg',
      frequency: 'As needed',
      start_date: Time.zone.today,
      end_date: 1.year.from_now.to_date,
      max_daily_doses: 4,
      min_hours_between_doses: 4,
      dose_cycle: :daily,
      schedule_type: :daily,
      schedule_config: {}
    )
  end

  def household_id = records.fetch(:household_id)
  def headers = records.fetch(:headers)
  def session_id = records.fetch(:session_id)
  def medication = records.fetch(:medication)
  def schedule = records.fetch(:schedule)
  def client_uuid = records.fetch(:client_uuid)

  def start_request(session)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        session.post(
          api_v1_household_sync_batches_path(household_id),
          params: batch_payload,
          headers: headers,
          as: :json
        )
        { status: session.response.status, body: session.response.parsed_body }
      end
    rescue StandardError => e
      e
    end
  end

  def join_worker(worker)
    raise Timeout::Error, 'timed out waiting for sync batch worker' unless worker.join(15)

    worker.value.tap { raise it if it.is_a?(StandardError) }
  end

  def batch_payload
    { batch: { operations: [batch_operation] } }
  end

  def batch_operation
    {
      action: 'create',
      resource_type: 'medication_take',
      attributes: batch_attributes
    }
  end

  def batch_attributes
    {
      client_uuid: client_uuid,
      source_type: 'schedule',
      source_id: schedule.portable_id,
      taken_at: 2.days.from_now.iso8601,
      taken_from_medication_id: medication.id
    }
  end
end
