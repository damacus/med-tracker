# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SchemaInventory do
  let(:expected_household_tables) do
    %w[
      people
      locations
      medications
      dosages
      schedules
      person_medications
      medication_takes
      notification_preferences
      health_events
      health_event_medications
      household_memberships
      person_access_grants
      household_invitations
      household_invitation_grants
      security_audit_events
      active_storage_attachments
    ]
  end
  let(:expected_global_tables) do
    %w[
      api_app_tokens
      api_sessions
      barcode_catalog_entries
      carer_relationships
      households
      native_device_tokens
      nhs_dmd_barcodes
      nhs_dmd_imports
      push_subscriptions
      users
    ]
  end

  it 'classifies tenant-owned and global catalogue tables explicitly' do
    expect(described_class.household_owned_tables).to include(*expected_household_tables)
    expect(described_class.global_tables).to include(*expected_global_tables)
  end

  it 'classifies every primary application table exactly once' do
    schema_tables = ActiveRecord::Base.connection.tables - %w[ar_internal_metadata schema_migrations]
    inventory_tables = described_class.household_owned_tables + described_class.global_tables

    expect(inventory_tables).to match_array(schema_tables)
    expect(inventory_tables.tally.select { |_table_name, count| count > 1 }).to be_empty
  end

  it 'keeps household-owned inventory entries as real database table names' do
    missing = described_class.household_owned_tables.reject do |table_name|
      ActiveRecord::Base.connection.table_exists?(table_name)
    end

    expect(missing).to be_empty
  end

  it 'fails when a household-owned table does not expose household_id' do
    missing = described_class.household_owned_tables.filter do |table_name|
      ActiveRecord::Base.connection.table_exists?(table_name) &&
        !ActiveRecord::Base.connection.column_exists?(table_name, :household_id)
    end

    expect(missing).to be_empty
  end

  it 'requires every household-owned table to enforce household_id at the database layer' do
    nullable = described_class.household_owned_tables.filter do |table_name|
      ActiveRecord::Base.connection.table_exists?(table_name) &&
        ActiveRecord::Base.connection.columns(table_name).find { it.name == 'household_id' }&.null
    end

    expect(nullable).to be_empty
  end
end
