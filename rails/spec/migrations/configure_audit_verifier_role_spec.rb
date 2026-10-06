# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260709150000_configure_audit_verifier_role')
require Rails.root.join('db/migrate/20260717130000_grant_audit_verifier_complete_visibility')

RSpec.describe ConfigureAuditVerifierRole do
  it 'can read audit evidence without reading clinical tables' do
    expect(privilege('audit_ledger_entries', 'SELECT')).to be(true)
    expect(privilege('security_audit_events', 'SELECT')).to be(true)
    expect(privilege('security_audit_events', 'INSERT')).to be(false)
    expect(privilege('versions', 'SELECT')).to be(true)
    expect(privilege('medications', 'SELECT')).to be(false)
  end

  it 'cannot alter source, ledger, checkpoint, or delivery history' do
    expect(privilege('security_audit_events', 'UPDATE')).to be(false)
    expect(privilege('versions', 'DELETE')).to be(false)
    expect(privilege('audit_ledger_entries', 'UPDATE')).to be(false)
    expect(privilege('audit_checkpoints', 'INSERT')).to be(false)
    expect(privilege('audit_export_deliveries', 'UPDATE')).to be(false)
  end

  it 'revokes verifier access when rolled back' do
    migration = described_class.new

    migration.down

    expect(privilege('audit_ledger_entries', 'SELECT')).to be(false)
    expect(privilege('security_audit_events', 'INSERT')).to be(false)
  ensure
    migration&.lock_verifier_privileges
    GrantAuditVerifierCompleteVisibility.new.up
  end

  def privilege(table_name, action)
    ActiveRecord::Base.connection.select_value(<<~SQL.squish)
      SELECT has_table_privilege('med_tracker_audit_verifier', '#{table_name}', '#{action}')
    SQL
  end
end
