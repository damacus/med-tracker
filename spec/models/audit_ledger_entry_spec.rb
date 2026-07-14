# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLedgerEntry do
  fixtures :accounts, :people, :users

  let(:account) { accounts(:admin) }
  let(:household) { users(:admin).person.household }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: account) do |record|
      record.person = users(:admin).person
      record.role = :owner
      record.status = :active
      record.joined_at = Time.current
    end
  end

  it 'chains security events to their exact source payload' do
    event = create_security_event('audit.test.first')
    entry = described_class.find_by!(source_table: 'security_audit_events', source_id: event.id)

    expect(entry).to have_attributes(
      household: household,
      sequence: 1,
      previous_hash: nil,
      hash_algorithm: 'sha256',
      schema_version: 1,
      retention_policy_version: 'clinical-security-v1'
    )
    expect(entry.entry_hash.bytesize).to eq(32)
    expect(entry.source_payload).to eq(source_payload_for(event))
    expect(entry.envelope.dig('source', 'table')).to eq('security_audit_events')
  end

  it 'links later entries to the previous entry hash' do
    first = create_security_event('audit.test.first')
    second = create_security_event('audit.test.second')
    first_entry = described_class.find_by!(source_table: 'security_audit_events', source_id: first.id)
    second_entry = described_class.find_by!(source_table: 'security_audit_events', source_id: second.id)

    expect(second_entry.sequence).to eq(first_entry.sequence + 1)
    expect(second_entry.previous_hash).to eq(first_entry.entry_hash)
    expect(AuditChainHead.find_by!(chain_key: "household:#{household.id}").last_hash).to eq(second_entry.entry_hash)
  end

  it 'uses a separate global chain for pre-household versions' do
    Audit::VersionEvent.record!(
      item_type: 'SystemAudit',
      item_id: account.id,
      event: 'system.test',
      object: { outcome: 'success' }
    )
    version = PaperTrail::Version.where(item_type: 'SystemAudit').last
    entry = described_class.find_by!(source_table: 'versions', source_id: version.id)

    expect(entry).to have_attributes(household_id: nil, chain_key: 'global')
  end

  it 'serializes concurrent inserts without duplicate or missing sequences' do
    household_id = household.id
    event_ids = Array.new(6) do |index|
      Thread.new do
        ActiveRecord::Base.connection_pool.with_connection do
          Audit::Event.record!(household_id:, event_type: "audit.concurrent.#{index}").id
        end
      end
    end.map(&:value)

    entries = described_class.where(source_table: 'security_audit_events', source_id: event_ids).order(:sequence)
    expect(entries.pluck(:sequence)).to eq((entries.first.sequence..entries.last.sequence).to_a)
    expect(entries.drop(1).map(&:previous_hash)).to eq(entries.take(5).map(&:entry_hash))
  end

  it 'denies runtime updates and deletes on audit source rows' do
    event = create_security_event('audit.runtime.source')

    expect(runtime_privilege('security_audit_events', 'UPDATE')).to be(false)
    expect(runtime_privilege('security_audit_events', 'DELETE')).to be(false)
    expect { with_runtime_role { execute_ledger("UPDATE security_audit_events SET event_type = 'audit.tampered'") } }
      .to raise_error(ActiveRecord::StatementInvalid)
    expect { with_runtime_role { event.delete } }.to raise_error(ActiveRecord::StatementInvalid)
  end

  it 'denies runtime inserts on ledger rows' do
    entry = runtime_ledger_entry
    expect(runtime_privilege('audit_ledger_entries', 'INSERT')).to be(false)
    expect do
      with_runtime_role { described_class.create!(entry.attributes.except('id')) }
    end.to raise_error(ActiveRecord::StatementInvalid)
  end

  it 'denies runtime updates on ledger rows' do
    entry = runtime_ledger_entry
    expect(runtime_privilege('audit_ledger_entries', 'UPDATE')).to be(false)

    expect do
      sql = "UPDATE audit_ledger_entries SET sequence = sequence + 1 WHERE id = #{entry.id}"
      with_runtime_role { execute_ledger(sql) }
    end.to raise_error(ActiveRecord::StatementInvalid)
  end

  it 'denies runtime deletes on ledger rows' do
    entry = runtime_ledger_entry
    expect(runtime_privilege('audit_ledger_entries', 'DELETE')).to be(false)

    expect do
      with_runtime_role { execute_ledger("DELETE FROM audit_ledger_entries WHERE id = #{entry.id}") }
    end.to raise_error(ActiveRecord::StatementInvalid)
  end

  it 'prevents household deletion from removing retained security evidence' do
    event = create_security_event('audit.retained')
    entry = described_class.find_by!(source_table: 'security_audit_events', source_id: event.id)

    expect(household.destroy).to be(false)
    expect(household.errors).to be_present
    expect(household.reload).to be_persisted
    expect(SecurityAuditEvent.where(id: event.id)).to exist
    expect(described_class.where(id: entry.id)).to exist
  end

  it 'exposes only the active household through the runtime read view' do
    own_event = create_security_event('audit.view.own')
    other_household = Household.create!(name: 'Other Audit Household', slug: 'other-audit-household')
    other_event = Audit::Event.record!(household: other_household, event_type: 'audit.view.other')
    own_entry = described_class.find_by!(source_table: 'security_audit_events', source_id: own_event.id)
    other_entry = described_class.find_by!(source_table: 'security_audit_events', source_id: other_event.id)

    visible_ids = with_runtime_role do
      ActiveRecord::Base.connection.execute(<<~SQL.squish)
        SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)
      SQL
      ActiveRecord::Base.connection.select_values('SELECT id FROM household_audit_ledger_entries ORDER BY id')
    end

    expect(visible_ids).to include(own_entry.id)
    expect(visible_ids).not_to include(other_entry.id)
    expect(runtime_privilege('audit_ledger_entries', 'SELECT')).to be(false)
  end

  def create_security_event(event_type)
    Audit::Event.record!(
      household: household,
      actor_account: account,
      actor_membership: membership,
      event_type: event_type,
      metadata: { outcome: 'success' }
    )
  end

  def with_runtime_role
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
    end
  end

  def source_payload_for(event)
    value = ActiveRecord::Base.connection.select_value(<<~SQL.squish)
      SELECT (to_jsonb(security_audit_events.*) - 'updated_at')::text
      FROM security_audit_events
      WHERE id = #{event.id}
    SQL
    JSON.parse(value)
  end

  def runtime_ledger_entry
    event = create_security_event('audit.runtime.ledger')
    described_class.find_by!(source_table: 'security_audit_events', source_id: event.id)
  end

  def execute_ledger(sql)
    ActiveRecord::Base.connection.execute(sql)
  end

  def runtime_privilege(table_name, privilege)
    ActiveRecord::Base.connection.select_value(<<~SQL.squish)
      SELECT has_table_privilege('med_tracker_app', '#{table_name}', '#{privilege}')
    SQL
  end
end
