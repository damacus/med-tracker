# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe 'API v1 idempotency concurrency' do
  self.use_transactional_tests = false

  let(:workers) { [] }
  let(:assignment_entered) { Queue.new }
  let(:release_assignment) { Queue.new }
  let(:records) { concurrency_records }

  before do
    version_ledger_trigger(:disable)
    allow(Audit::Event).to receive(:record!)
    allow(AuthTokenAuditLogger).to receive(:new)
      .and_return(instance_double(AuthTokenAuditLogger, record: nil))
    records
  end

  after do
    release_assignment << true
    terminate_workers
    cleanup_records
    version_ledger_trigger(:enable)
  end

  it 'serializes concurrent People creates before mutation side effects' do
    pause_first_assignment
    sessions = Array.new(2) { ActionDispatch::Integration::Session.new(Rails.application) }
    first = start_request(sessions.first, records.fetch(:tokens).first)
    wait_for(assignment_entered, 'first People assignment')
    second = start_request(sessions.second, records.fetch(:tokens).second)
    second_pid = wait_for(second.fetch(:pid), 'second request database session')
    wait_for_advisory_wait(second_pid)

    release_assignment << true
    responses = [join_worker(first.fetch(:worker)), join_worker(second.fetch(:worker))]
    person = Person.find_by!(household_id: records.fetch(:household_id), name: records.fetch(:name))

    expect(responses.map { it.fetch(:status) }).to eq([201, 201])
    expect(responses.map { it.fetch(:body) }.uniq.one?).to be true
    expect(responses.count { it.fetch(:replayed) }).to eq(1)
    expect(Person.where(household_id: records.fetch(:household_id), name: records.fetch(:name)).count).to eq(1)
    expect(CarerRelationship.where(patient: person).count).to eq(1)
    expect(PersonAccessGrant.where(person: person, revoked_at: nil).count).to eq(1)
    expect(PaperTrail::Version.where(item_type: 'Person', item_id: person.id, event: 'create').count).to eq(1)
    expect(ApiIdempotencyKey.where(household_id: records.fetch(:household_id), key: records.fetch(:key)).count).to eq(1)
  end

  def pause_first_assignment
    gate = Mutex.new
    paused = false
    allow(CareDelegation::Assign).to receive(:new).and_wrap_original do |original, *arguments, **keywords|
      service = original.call(*arguments, **keywords)
      should_pause = gate.synchronize do
        next false if paused

        paused = true
      end
      pause_assignment(service) if should_pause
      service
    end
  end

  def pause_assignment(service)
    original_call = service.method(:call)
    allow(service).to receive(:call) do
      assignment_entered << true
      wait_for(release_assignment, 'People assignment release')
      original_call.call
    end
  end

  def start_request(session, token)
    pid = Queue.new
    worker = Thread.new { perform_request(session, token, pid) }
    workers << worker
    { worker: worker, pid: pid }
  end

  def perform_request(session, token, pid)
    ActiveRecord::Base.connection_pool.with_connection do
      pid << ActiveRecord::Base.connection.select_value('SELECT pg_backend_pid()').to_i
      post_person(session, token)
      response_result(session.response)
    end
  rescue StandardError => e
    e
  end

  def post_person(session, token)
    session.post(
      api_v1_household_people_path(records.fetch(:household_id)),
      params: person_payload,
      headers: api_auth_headers(token).merge('Idempotency-Key' => records.fetch(:key)),
      as: :json
    )
  end

  def response_result(request_response)
    {
      status: request_response.status,
      body: request_response.parsed_body,
      replayed: request_response.headers['Idempotency-Replayed'] == 'true'
    }
  end

  def join_worker(worker)
    raise Timeout::Error, 'timed out waiting for idempotency request' unless worker.join(thread_timeout)

    worker.value.tap { raise it if it.is_a?(StandardError) }
  end

  def wait_for(queue, description)
    Timeout.timeout(thread_timeout) { queue.pop }
  rescue Timeout::Error
    raise Timeout::Error, "timed out waiting for #{description}"
  end

  def wait_for_advisory_wait(pid)
    Timeout.timeout(thread_timeout) do
      loop do
        return if advisory_waiting?(pid)

        Thread.pass
      end
    end
  rescue Timeout::Error
    raise Timeout::Error, "timed out waiting for database session #{pid} to block on idempotency"
  end

  def advisory_waiting?(pid)
    ActiveRecord::Base.connection.select_value(
      ActiveRecord::Base.sanitize_sql_array(
        ["SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid = ? AND locktype = 'advisory' AND NOT granted)", pid]
      )
    )
  end

  def concurrency_records
    household = create_concurrency_household
    account = create_concurrency_account
    person = create_concurrency_owner(account, household)
    create_concurrency_user(account, person)
    membership = create_concurrency_membership(account, person, household)
    sessions_and_tokens = issue_sessions(account, membership)
    {
      household_id: household.id,
      account_id: account.id,
      key: SecureRandom.uuid,
      name: "Concurrent API Dependent #{SecureRandom.hex(4)}",
      session_ids: sessions_and_tokens.map(&:first),
      tokens: sessions_and_tokens.map(&:last)
    }
  end

  def create_concurrency_household
    Household.create!(
      name: 'Idempotency Concurrency Household',
      slug: "idempotency-concurrency-#{SecureRandom.hex(4)}"
    )
  end

  def create_concurrency_account
    Account.create!(
      email: "idempotency-concurrency-#{SecureRandom.hex(4)}@example.test",
      status: :verified
    )
  end

  def create_concurrency_owner(account, household)
    Person.create!(
      household: household,
      account: account,
      name: 'Idempotency Concurrency Owner',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def create_concurrency_user(account, person)
    User.create!(
      person: person,
      email_address: account.email,
      password: 'password',
      active: true
    )
  end

  def create_concurrency_membership(account, person, household)
    household.household_memberships.create!(
      account: account,
      person: person,
      role: :owner,
      status: :active
    )
  end

  def issue_sessions(account, membership)
    Array.new(2) do
      session, token = ApiSession.issue_for(
        account: account,
        household_membership: membership,
        device_name: 'RSpec idempotency client'
      )
      [session.id, token]
    end
  end

  def person_payload
    {
      person: {
        name: records.fetch(:name),
        date_of_birth: 8.years.ago.to_date,
        person_type: 'minor'
      }
    }
  end

  def version_ledger_trigger(action)
    ActiveRecord::Base.connection.execute(
      "ALTER TABLE versions #{action.to_s.upcase} TRIGGER append_versions_to_audit_ledger"
    )
  end

  def terminate_workers
    workers.each do |worker|
      worker.kill if worker.alive?
      worker.join(1)
    end
  end

  def cleanup_records
    created_people = Person.where(household_id: records.fetch(:household_id))
    person_ids = created_people.ids
    user_ids = User.where(person_id: person_ids).ids
    ApiChangeEvent.where(household_id: records.fetch(:household_id)).delete_all
    cleanup_person_records(person_ids)
    cleanup_request_records
    cleanup_identity_records(created_people, user_ids)
  end

  def cleanup_identity_records(created_people, user_ids)
    cleanup_versions
    cleanup_people(created_people, user_ids)
    cleanup_household
  end

  def cleanup_versions
    PaperTrail::Version.where(household_id: records.fetch(:household_id)).delete_all
  end

  def cleanup_people(created_people, user_ids)
    HouseholdMembership.where(household_id: records.fetch(:household_id)).delete_all
    User.where(id: user_ids).delete_all
    created_people.delete_all
  end

  def cleanup_household
    Location.where(household_id: records.fetch(:household_id)).delete_all
    Household.where(id: records.fetch(:household_id)).delete_all
    Account.where(id: records.fetch(:account_id)).delete_all
  end

  def cleanup_person_records(person_ids)
    PersonAccessGrant.where(person_id: person_ids).delete_all
    CarerRelationship.where(patient_id: person_ids).or(CarerRelationship.where(carer_id: person_ids)).delete_all
    LocationMembership.where(person_id: person_ids).delete_all
  end

  def cleanup_request_records
    ApiIdempotencyKey.where(household_id: records.fetch(:household_id), key: records.fetch(:key)).delete_all
    ApiSession.where(id: records.fetch(:session_ids)).delete_all
  end

  def thread_timeout = 10
end
