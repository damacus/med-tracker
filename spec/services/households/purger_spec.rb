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

  it 'retains immutable audit history and appends a non-PHI purge tombstone under the runtime role',
     :aggregate_failures do
    member_account = account('hosted-purge-audited-member@example.test')
    actor_membership = household.household_memberships.create!(account: member_account, role: :member, status: :active)
    retained_event = Audit::Event.record!(
      household: household,
      actor_account: member_account,
      actor_membership: actor_membership,
      event_type: 'purge.runtime.seed',
      metadata: { outcome: 'success' }
    )
    retained_version = PaperTrail::Version.create!(
      actor_membership_id: actor_membership.id,
      audit_context: {},
      event: 'update',
      household_id: household.id,
      item_id: 123_456,
      item_type: 'Person',
      object: '{}'
    )

    with_runtime_role do
      expect_direct_audit_delete_denied
      expect_direct_version_update_denied(retained_version)

      run = described_class.call(household: household, actor_account: operator)

      expect(run).to be_completed
      expect(household.reload).to be_lifecycle_purged
      set_runtime_household
      expect(SecurityAuditEvent.where(id: retained_event.id, household: household)).to exist
      expect(SecurityAuditEvent.find(retained_event.id).actor_membership_id).to eq(actor_membership.id)
      expect(PaperTrail::Version.find(retained_version.id).actor_membership_id).to eq(actor_membership.id)
      expect(HouseholdMembership.where(id: actor_membership.id)).not_to exist
      expect(purge_tombstone.metadata).to eq(
        'attempts' => 1,
        'household_id' => household.id,
        'last_completed_table' => 'people',
        'outcome' => 'success',
        'purge_run_id' => run.id
      )
    end
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
    expect(described_class.call(household: household, actor_account: operator).id).to eq(failed_run.id)
    verify_completed_state(failed_run)
    verify_purgeable_inventory_empty
    verify_storage(records)
    verify_uncached_attachment_check(records.fetch(:purged_blob))
    expect(SecurityAuditEvent.where(household: household)).to exist
    expect(SecurityAuditEvent.where(household: household, event_type: 'household.purge.completed').count).to eq(1)
    expect(ledger_events('household.purge.completed')).to exist
  end

  def verify_completed_state(failed_run)
    expect([failed_run.reload.status, household.reload.lifecycle_state, ledger_events('household.offboarded').count])
      .to eq(['completed', 'purged', 1])
  end

  def verify_uncached_attachment_check(blob)
    expect(ActiveStorage::Attachment).to have_received(:exists?).with(blob_id: blob.id).at_least(:once)
  end

  def verify_purgeable_inventory_empty
    SchemaInventory.purgeable_household_owned_tables.each do |table_name|
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

  def purge_tombstone
    SecurityAuditEvent.where(household: household, event_type: 'household.purge.completed').sole
  end

  def with_runtime_role
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
      raise ActiveRecord::Rollback
    end
  end

  def expect_direct_audit_delete_denied
    expect do
      ActiveRecord::Base.connection.transaction(requires_new: true) do
        ActiveRecord::Base.connection.exec_delete(
          "DELETE FROM security_audit_events WHERE household_id = #{household.id}"
        )
      end
    end.to raise_error(ActiveRecord::StatementInvalid)
  end

  def expect_direct_version_update_denied(version)
    expect do
      ActiveRecord::Base.connection.transaction(requires_new: true) do
        ActiveRecord::Base.connection.exec_update(
          "UPDATE versions SET actor_membership_id = NULL WHERE id = #{version.id}"
        )
      end
    end.to raise_error(ActiveRecord::StatementInvalid)
  end

  def set_runtime_household
    ActiveRecord::Base.connection.execute(
      "SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)"
    )
  end
end
