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

  it 'ends active support access before purging the household' do
    support_session = SupportAccessSession.create!(
      platform_admin: operator.platform_admin,
      household: household,
      reason: 'Investigate household before purge',
      mfa_verified_at: Time.current
    )

    expect do
      described_class.call(household: household, actor_account: operator)
    end.to change { support_session.reload.ended_at }.from(nil)

    expect(household.reload).to be_lifecycle_purged
  end

  it 'preserves a shared login identity while invalidating prior API credentials under forced RLS',
     :aggregate_failures do
    shared = shared_login_records

    with_runtime_role do
      described_class.call(household: household, actor_account: operator)
      set_runtime_household(other_household)

      expect(shared_login_state(shared)).to eq(expected_shared_login_state(shared))
    end
  end

  def shared_login_records
    shared_account = account('shared-purge-member@example.test')
    person = create(:person, household: household, account: shared_account, name: 'Shared Purge Member')
    user = User.create!(person: person, email_address: shared_account.email, password: 'password')
    household.household_memberships.create!(account: shared_account, person: person, role: :owner, status: :active)
    other_membership = other_household.household_memberships.create!(
      account: shared_account,
      role: :owner,
      status: :active
    )
    api_session = ApiSession.issue_for(account: shared_account, household_membership: other_membership).first
    preference = create(:notification_preference, household: household, person: person)

    { account: shared_account, user: user, person: person, membership: other_membership,
      api_session: api_session, preference: preference }
  end

  def expected_shared_login_state(shared)
    {
      identity_replaced: true,
      identity_household: other_household,
      identity_account: shared.fetch(:account),
      identity_location_count: 0,
      account_resolves_user: true,
      membership_identity_linked: true,
      membership_status: 'active',
      api_session_active: false,
      target_preference_exists: false,
      target_person_exists: false
    }
  end

  def shared_login_state(shared)
    user = shared.fetch(:user).reload
    identity_person = user.person

    surviving_identity_state(shared, user, identity_person)
      .merge(surviving_access_state(shared, identity_person), target_clinical_data_state(shared))
  end

  def surviving_identity_state(shared, user, identity_person)
    {
      identity_replaced: identity_person != shared.fetch(:person),
      identity_household: identity_person.household,
      identity_account: identity_person.account,
      identity_location_count: identity_person.location_memberships.size,
      account_resolves_user: shared.fetch(:account).reload.person.user == user
    }
  end

  def surviving_access_state(shared, identity_person)
    membership = shared.fetch(:membership).reload
    {
      membership_identity_linked: membership.person == identity_person,
      membership_status: membership.status,
      api_session_active: shared.fetch(:api_session).reload.active_for_membership?
    }
  end

  def target_clinical_data_state(shared)
    set_runtime_household(household)
    {
      target_preference_exists: NotificationPreference.exists?(id: shared.fetch(:preference).id),
      target_person_exists: Person.exists?(household: household)
    }
  end

  it 'retains immutable audit history and appends a non-PHI purge tombstone under the runtime role',
     :aggregate_failures do
    history = retained_audit_history

    with_runtime_role do
      expect(verify_runtime_purge(history)).to be_completed
    end
  end

  def retained_audit_history
    member_account = account('hosted-purge-audited-member@example.test')
    actor_membership = household.household_memberships.create!(account: member_account, role: :member, status: :active)
    {
      membership: actor_membership,
      event: retained_security_event(member_account, actor_membership),
      version: retained_version(actor_membership)
    }
  end

  def retained_security_event(member_account, actor_membership)
    Audit::Event.record!(
      household: household,
      actor_account: member_account,
      actor_membership: actor_membership,
      event_type: 'purge.runtime.seed',
      metadata: { outcome: 'success' }
    )
  end

  def retained_version(actor_membership)
    PaperTrail::Version.create!(
      actor_membership_id: actor_membership.id,
      audit_context: {},
      event: 'update',
      household_id: household.id,
      item_id: 123_456,
      item_type: 'Person',
      object: '{}'
    )
  end

  def verify_runtime_purge(history)
    expect_direct_audit_delete_denied
    expect_direct_version_update_denied(history.fetch(:version))
    run = described_class.call(household: household, actor_account: operator)

    verify_runtime_purge_state(run, history)
    run
  end

  def verify_runtime_purge_state(run, history)
    expect(household.reload).to be_lifecycle_purged
    set_runtime_household
    verify_retained_audit_history(history)
    expect(HouseholdMembership.where(id: history.fetch(:membership).id)).not_to exist
    verify_purge_tombstone(run)
  end

  def verify_retained_audit_history(history)
    verify_retained_security_event(history)
    verify_retained_version(history)
  end

  def verify_retained_security_event(history)
    event = history.fetch(:event)
    membership_id = history.fetch(:membership).id

    expect(SecurityAuditEvent.where(id: event.id, household: household)).to exist
    expect(SecurityAuditEvent.find(event.id).actor_membership_id).to eq(membership_id)
  end

  def verify_retained_version(history)
    expect(PaperTrail::Version.find(history.fetch(:version).id).actor_membership_id)
      .to eq(history.fetch(:membership).id)
  end

  def verify_purge_tombstone(run)
    expect(purge_tombstone.metadata).to eq(
      'attempts' => 1,
      'household_id' => household.id,
      'last_completed_table' => 'people',
      'outcome' => 'success',
      'purge_run_id' => run.id
    )
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
    verify_resumed_run(failed_run)
    verify_completed_state(failed_run)
    verify_purged_data(records)
    verify_purge_evidence
  end

  def verify_resumed_run(failed_run)
    expect(described_class.call(household: household, actor_account: operator).id).to eq(failed_run.id)
  end

  def verify_purged_data(records)
    verify_purgeable_inventory_empty
    verify_storage(records)
    verify_uncached_attachment_check(records.fetch(:purged_blob))
  end

  def verify_purge_evidence
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

  def set_runtime_household(target_household = household)
    ActiveRecord::Base.connection.execute(
      "SELECT set_config('med_tracker.current_household_id', '#{target_household.id}', true)"
    )
  end
end
