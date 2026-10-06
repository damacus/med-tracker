# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260709143000_configure_audit_object_lock_exporter')

RSpec.describe ConfigureAuditObjectLockExporter do
  it 'can read ledger evidence and update delivery state without reading clinical tables' do
    expect(privilege('audit_ledger_entries', 'SELECT')).to be(true)
    expect(privilege('audit_export_deliveries', 'UPDATE')).to be(true)
    expect(privilege('audit_ledger_entries', 'UPDATE')).to be(false)
    expect(privilege('medications', 'SELECT')).to be(false)
    expect(privilege('versions', 'SELECT')).to be(false)
  end

  it 'cannot directly insert or alter signing evidence' do
    expect(privilege('audit_signing_keys', 'INSERT')).to be(false)
    expect(privilege('audit_checkpoints', 'INSERT')).to be(false)
    expect(privilege('audit_checkpoints', 'UPDATE')).to be(false)
  end

  def privilege(table_name, action)
    ActiveRecord::Base.connection.select_value(<<~SQL.squish)
      SELECT has_table_privilege('med_tracker_audit_exporter', '#{table_name}', '#{action}')
    SQL
  end
end
