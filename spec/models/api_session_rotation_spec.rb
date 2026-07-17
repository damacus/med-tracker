# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe ApiSession do
  self.use_transactional_tests = false
  fixtures :accounts, :people, :users

  before do
    audit_logger = instance_double(AuthTokenAuditLogger, record: nil)
    allow(AuthTokenAuditLogger).to receive(:new).and_return(audit_logger)
  end

  it 'allows exactly one concurrent refresh-token rotation' do
    records = rotation_records
    outcomes = concurrent_rotations(issue_refresh_token(records))

    expect(outcomes.count(&:present?)).to eq(1)
  ensure
    cleanup_rotation_records(records) if records
  end

  it 'persists exactly one concurrent stale last-used touch' do
    records = rotation_records
    session, = described_class.issue_for(
      account: records.fetch(:account),
      household_membership: records.fetch(:membership)
    )
    session.update_column(:last_used_at, 6.minutes.ago)
    stale_sessions = 2.times.map { described_class.find(session.id) }
    ready = Queue.new
    start = Queue.new
    results = Queue.new
    threads = stale_sessions.map do |stale_session|
      Thread.new do
        ActiveRecord::Base.connection_pool.with_connection do
          ready << true
          start.pop
          results << stale_session.touch_last_used!
        end
      end
    end

    release_rotations(threads, ready, start)

    expect(2.times.map { Timeout.timeout(10) { results.pop } }.count(true)).to eq(1)
  ensure
    Array(threads).each { it.kill if it&.alive? }
    cleanup_rotation_records(records) if records
  end

  def issue_refresh_token(records)
    described_class.issue_for(
      account: records.fetch(:account),
      household_membership: records.fetch(:membership)
    ).last
  end

  def concurrent_rotations(refresh_token)
    ready = Queue.new
    start = Queue.new
    results = Queue.new
    threads = 2.times.map { rotation_thread(refresh_token, ready, start, results) }

    release_rotations(threads, ready, start)
    2.times.map { Timeout.timeout(10) { results.pop } }
  ensure
    Array(threads).each { it.kill if it&.alive? }
  end

  def release_rotations(threads, ready, start)
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    threads.each { expect(it.join(10)).to be_truthy }
  end

  def rotation_thread(refresh_token, ready, start, results)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        ready << true
        start.pop
        results << described_class.rotate_refresh_token(refresh_token)
      end
    end
  end

  def rotation_records
    account = accounts(:jane_doe)
    AccountLockout.where(account_id: account.id).delete_all
    household = Household.create!(name: "Rotation #{SecureRandom.hex(4)}")
    membership = household.household_memberships.create!(
      account: account,
      role: :owner,
      status: :active
    )
    {
      account: account,
      household: household,
      membership: membership
    }
  end

  def cleanup_rotation_records(records)
    ApiSession.where(account: records.fetch(:account)).delete_all
    HouseholdMembership.where(id: records.fetch(:membership).id).delete_all
    Household.where(id: records.fetch(:household).id).delete_all
  end
end
