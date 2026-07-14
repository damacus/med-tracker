# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::ExportExpiryProcessor do
  include ActiveSupport::Testing::TimeHelpers

  let(:expiry_time) { Time.zone.parse('2026-07-13 12:00:00') }
  let(:household) { create(:household) }
  let(:requester) { Account.create!(email: 'expiry-owner@example.test', status: :verified) }
  let(:membership) do
    household.household_memberships.create!(account: requester, role: :owner, status: :active)
  end

  it 'deletes an artifact at the retention boundary and remains idempotent on retry', :aggregate_failures do
    due_export = ready_export(expires_at: expiry_time)
    future_export = ready_export(expires_at: expiry_time + 1.second)
    blob = due_export.artifact.blob
    blob.attachments.load

    travel_to(expiry_time) do
      expect(described_class.call).to eq(1)
      expect(described_class.call).to eq(0)
    end

    expect(due_export.reload).to be_expired
    expect(due_export.artifact).not_to be_attached
    expect(ActiveStorage::Blob.where(id: blob.id)).not_to exist
    expect(future_export.reload).to be_ready
    events = SecurityAuditEvent.where(household: household, event_type: 'household.export.expired')
    expect(events.count).to eq(1)
    expect(events.sole.metadata).to eq('export_id' => due_export.id, 'outcome' => 'expired')
  end

  it 'preserves due artifacts while an active household retention hold exists' do
    due_export = ready_export(expires_at: expiry_time)
    operator = Account.create!(email: 'expiry-operator@example.test', status: :verified)
    PlatformAdmin.create!(account: operator)
    Households::RetentionHoldManager.place!(
      household: household, actor_account: operator, reason: 'Approved preservation',
      review_on: expiry_time.to_date + 30.days
    )

    travel_to(expiry_time) { expect(described_class.call).to eq(0) }

    expect(due_export.reload).to be_ready
    expect(due_export.artifact).to be_attached
  end

  it 'rechecks retention holds inside the household transaction before deleting an artifact' do
    due_export = ready_export(expires_at: expiry_time)
    allow(Household).to receive(:find_each).and_return([household])
    allow(household).to receive(:with_lock) do |&block|
      create_retention_hold
      block.call
    end

    travel_to(expiry_time) { expect(described_class.call).to eq(0) }

    expect(due_export.reload).to be_ready
    expect(due_export.artifact).to be_attached
  end

  it 'expires stale generating archives after the configured timeout but preserves live generation' do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('HOUSEHOLD_EXPORT_GENERATION_TIMEOUT_MINUTES', '60').and_return('15')
    stale_export = generating_export(started_at: expiry_time - 16.minutes)
    live_export = generating_export(started_at: expiry_time - 14.minutes)

    travel_to(expiry_time) { expect(described_class.call).to eq(1) }

    expect(stale_export.reload).to be_expired
    expect(stale_export.artifact).not_to be_attached
    expect(live_export.reload).to be_generating
    expect(live_export.artifact).to be_attached
  end

  def ready_export(expires_at:)
    export = HouseholdExport.create!(
      household: household, requested_by_account: requester, requested_at: 1.day.ago,
      expires_at: expires_at, status: :ready, ready_at: 1.hour.ago
    )
    export.artifact.attach(
      io: StringIO.new("export-#{expires_at.to_f}"), filename: "export-#{export.id}.zip",
      content_type: 'application/zip'
    )
    membership
    export
  end

  def generating_export(started_at:)
    export = HouseholdExport.create!(
      household: household,
      requested_by_account: requester,
      requested_at: started_at,
      expires_at: expiry_time + 1.day,
      status: :generating,
      generation_started_at: started_at
    )
    export.artifact.attach(
      io: StringIO.new("generating-export-#{started_at.to_f}"),
      filename: "export-#{export.id}.zip",
      content_type: 'application/zip'
    )
    membership
    export
  end

  def create_retention_hold
    operator = Account.create!(email: 'concurrent-hold-operator@example.test', status: :verified)
    household.household_retention_holds.create!(
      approved_by_account: operator,
      reason: 'Concurrent preservation request',
      review_on: expiry_time.to_date + 30.days,
      placed_at: expiry_time
    )
  end
end
