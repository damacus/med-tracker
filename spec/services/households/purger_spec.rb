# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::Purger do
  def account(email)
    Account.create!(email: email, status: :verified)
  end

  let(:household) { create(:household) }
  let(:other_household) { create(:household) }
  let(:operator) { account('hosted-purge-operator@example.test') }

  before { PlatformAdmin.create!(account: operator) }

  it 'refuses purge while an active retention hold exists' do
    Households::RetentionHoldManager.place!(
      household:,
      actor_account: operator,
      reason: 'Active preservation request',
      review_on: 1.month.from_now.to_date
    )

    expect do
      described_class.call(household:, actor_account: operator)
    end.to raise_error(described_class::ActiveRetentionHold)
  end

  it 'requires an active platform administrator' do
    unauthorized_account = account('hosted-purge-unauthorized@example.test')

    expect do
      described_class.call(household:, actor_account: unauthorized_account)
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'resumes after a partial failure and deletes every tenant inventory row and owned attachment only',
     :aggregate_failures do
    records = purge_records
    spy_on_attachment_checks
    expect_interrupted_purge
    failed_run = verify_partial_progress(records)
    described_class.call(household:, actor_account: operator)
    verify_completed_purge(failed_run, records)
  end

  def purge_records
    person = create(:person, household:)
    notification_preference = create(:notification_preference, person: person, household: household)
    medication = create(:medication, household: household)
    person.avatar.attach(io: StringIO.new('purged-bytes'), filename: 'purged.png', content_type: 'image/png')
    person.avatar.blob.attachments.load
    target_records(notification_preference, medication, person).merge(preserved_records)
  end

  def target_records(notification_preference, medication, person)
    { notification_preference: notification_preference, medication: medication, purged_blob: person.avatar.blob }
  end

  def spy_on_attachment_checks = allow(ActiveStorage::Attachment).to receive(:exists?).and_call_original

  def preserved_records
    other_person = create(:person, household: other_household)
    other_person.avatar.attach(
      io: StringIO.new('preserved-bytes'), filename: 'preserved.png', content_type: 'image/png'
    )
    { preserved_blob: other_person.avatar.blob, other_person: other_person }
  end

  def expect_interrupted_purge
    callback = lambda do |table_name|
      next unless table_name == 'notification_preferences'

      raise 'injected purge interruption'
    end

    expect do
      described_class.call(household:, actor_account: operator, after_table: callback)
    end.to raise_error('injected purge interruption')
  end

  def verify_partial_progress(records)
    failed_run = HouseholdPurgeRun.where(household:).sole
    preference = NotificationPreference.where(id: records.fetch(:notification_preference).id)
    medication = Medication.where(id: records.fetch(:medication).id)
    expect([failed_run.status, failed_run.last_completed_table, preference.exists?, medication.exists?])
      .to eq(['failed', 'notification_preferences', false, true])
    failed_run
  end

  def verify_completed_purge(failed_run, records)
    verify_completed_state(failed_run)
    verify_inventory_empty
    verify_storage(records)
    verify_uncached_attachment_check(records.fetch(:purged_blob))
    expect(ledger_events('household.purge.completed')).to exist
  end

  def verify_completed_state(failed_run)
    expect([failed_run.reload.status, household.reload.lifecycle_state, ledger_events('household.offboarded').count])
      .to eq(['completed', 'purged', 1])
  end

  def verify_uncached_attachment_check(blob)
    expect(ActiveStorage::Attachment).to have_received(:exists?).with(blob_id: blob.id).at_least(:once)
  end

  def verify_inventory_empty
    SchemaInventory.household_owned_tables.each do |table_name|
      expect(ActiveRecord::Base.connection.select_value(<<~SQL.squish)).to eq(0)
        SELECT COUNT(*) FROM #{ActiveRecord::Base.connection.quote_table_name(table_name)}
        WHERE household_id = #{household.id}
      SQL
    end
  end

  def verify_storage(records)
    expect(ActiveStorage::Blob.where(id: records.fetch(:purged_blob).id)).not_to exist
    expect(ActiveStorage::Blob.where(id: records.fetch(:preserved_blob).id)).to exist
    expect(records.fetch(:other_person).reload).to be_present
  end

  def ledger_events(event_type)
    AuditLedgerEntry.where(household: household).where("envelope ->> 'event_type' = ?", event_type)
  end
end
