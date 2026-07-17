# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260717130000_grant_audit_verifier_complete_visibility')

RSpec.describe GrantAuditVerifierCompleteVisibility do
  let(:connection) { ActiveRecord::Base.connection }
  let(:migration) { described_class.new }

  it 'gives only the verifier complete read visibility while FORCE RLS remains enabled' do
    policy = connection.select_one(<<~SQL.squish)
      SELECT cmd, roles, qual, with_check
      FROM pg_policies
      WHERE schemaname = 'public'
        AND tablename = 'security_audit_events'
        AND policyname = 'audit_verifier_complete_visibility'
    SQL

    expect(policy).to eq(
      'cmd' => 'SELECT', 'roles' => '{med_tracker_audit_verifier}', 'qual' => 'true', 'with_check' => nil
    )
    expect(connection.select_value(<<~SQL.squish)).to be(true)
      SELECT relrowsecurity AND relforcerowsecurity
      FROM pg_class
      WHERE oid = 'security_audit_events'::regclass
    SQL
  end

  it 'does not grant clinical reads or row-security bypass' do
    expect(privilege('medications', 'SELECT')).to be(false)
    expect(connection.select_value(<<~SQL.squish)).to be(true)
      SELECT NOT rolsuper AND NOT rolbypassrls
      FROM pg_roles
      WHERE rolname = 'med_tracker_audit_verifier'
    SQL
  end

  it 'does not grant audit mutation' do
    expect(privilege('security_audit_events', 'INSERT')).to be(false)
    expect(privilege('security_audit_events', 'UPDATE')).to be(false)
    expect(privilege('security_audit_events', 'DELETE')).to be(false)
    expect(privilege('audit_ledger_entries', 'UPDATE')).to be(false)
  end

  it 'removes and restores only the complete-visibility policy across rollback' do
    migration.down

    expect(policy_exists?).to be(false)
    expect(privilege('security_audit_events', 'INSERT')).to be(false)

    migration.up
    expect(policy_exists?).to be(true)
  ensure
    migration&.up unless policy_exists?
  end

  it 'keeps schema loading safe when the optional verifier role is absent' do
    schema = Rails.root.join('db/schema.rb').read

    expect(schema).to include("IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_audit_verifier')")
    expect(schema).to include('CREATE POLICY audit_verifier_complete_visibility')
    expect(schema).to include('FOR SELECT TO med_tracker_audit_verifier')
  end

  def privilege(table_name, action)
    connection.select_value(<<~SQL.squish)
      SELECT has_table_privilege('med_tracker_audit_verifier', '#{table_name}', '#{action}')
    SQL
  end

  def policy_exists?
    connection.select_value(<<~SQL.squish).to_i.positive?
      SELECT COUNT(*)
      FROM pg_policies
      WHERE tablename = 'security_audit_events'
        AND policyname = 'audit_verifier_complete_visibility'
    SQL
  end
end
