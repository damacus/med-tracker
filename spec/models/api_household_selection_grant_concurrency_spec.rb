# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe ApiHouseholdSelectionGrant do
  self.use_transactional_tests = false
  fixtures :accounts, :people, :users

  before do
    audit_logger = instance_double(AuthTokenAuditLogger, record: nil)
    allow(AuthTokenAuditLogger).to receive(:new).and_return(audit_logger)
  end

  it 'allows exactly one concurrent household selection' do
    records = selection_records
    grant, token = described_class.issue_for(account: records.fetch(:account))
    outcomes = concurrent_selections(token, records.fetch(:membership).household_id)

    expect(outcomes.count { it.is_a?(described_class::Result) }).to eq(1)
    expect(outcomes.count(:invalid)).to eq(1)
    expect(
      ApiSession.where(
        account: records.fetch(:account),
        household_membership: records.fetch(:membership)
      ).count
    ).to eq(1)
    expect(grant.reload.used_at).to be_present
  ensure
    cleanup_selection_records(records, grant)
  end

  def concurrent_selections(token, household_id)
    ready = Queue.new
    start = Queue.new
    results = Queue.new
    threads = 2.times.map do
      Thread.new do
        ActiveRecord::Base.connection_pool.with_connection do
          ready << true
          start.pop
          result = described_class.select_household(token: token, household_id: household_id)
          results << result
        rescue described_class::InvalidGrant
          results << :invalid
        end
      end
    end

    release_selections(threads, ready, start)
    2.times.map { Timeout.timeout(10) { results.pop } }
  ensure
    Array(threads).each { it.kill if it&.alive? }
  end

  def release_selections(threads, ready, start)
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    threads.each { expect(it.join(10)).to be_truthy }
  end

  def selection_records
    account = accounts(:jane_doe)
    AccountLockout.where(account_id: account.id).delete_all
    household = Household.create!(name: "Selection #{SecureRandom.hex(4)}")
    membership = household.household_memberships.create!(
      account: account,
      role: :owner,
      status: :active
    )
    { account: account, household: household, membership: membership }
  end

  def cleanup_selection_records(records, grant)
    ApiSession.where(household_membership_id: records&.dig(:membership)&.id).delete_all
    ApiHouseholdSelectionGrant.where(id: grant&.id).delete_all
    HouseholdMembership.where(id: records&.dig(:membership)&.id).delete_all
    Household.where(id: records&.dig(:household)&.id).delete_all
  end
end
