# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'EnforceStrictHouseholdTenantBoundary' do
  delegate :connection, to: :'ActiveRecord::Base'

  before do
    unless defined?(EnforceStrictHouseholdTenantBoundary)
      load Rails.root.join('db/migrate/20260624091000_enforce_strict_household_tenant_boundary.rb')
    end
  end

  around do |example|
    connection.transaction(requires_new: true) do
      example.run
      raise ActiveRecord::Rollback
    end
  end

  it 'relaxes forced RLS before legacy household backfill runs' do
    migration = EnforceStrictHouseholdTenantBoundary.new
    table_name = 'people'

    connection.execute("ALTER TABLE #{connection.quote_table_name(table_name)} FORCE ROW LEVEL SECURITY")

    expect { migration.send(:relax_forced_rls_for_backfill) }
      .to change { forced_rls?(table_name) }
      .from(true)
      .to(false)
  end

  def forced_rls?(table_name)
    connection.select_value(<<~SQL.squish)
      SELECT relforcerowsecurity
      FROM pg_class
      WHERE oid = #{connection.quote(table_name)}::regclass
    SQL
  end
end
