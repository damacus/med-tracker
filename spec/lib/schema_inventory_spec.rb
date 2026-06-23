# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SchemaInventory do
  let(:expected_household_tables) do
    %w[
      people
      locations
      medications
      medication_dosage_options
      dosages
      schedules
      person_medications
      medication_takes
      notification_preferences
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
      barcode_catalog_entries
      nhs_dmd_barcodes
      nhs_dmd_imports
    ]
  end

  it 'classifies tenant-owned and global catalogue tables explicitly' do
    expect(described_class.household_owned_tables).to include(*expected_household_tables)
    expect(described_class.global_tables).to include(*expected_global_tables)
  end

  it 'fails when a household-owned table does not expose household_id' do
    missing = described_class.household_owned_tables.filter do |table_name|
      ActiveRecord::Base.connection.table_exists?(table_name) &&
        !ActiveRecord::Base.connection.column_exists?(table_name, :household_id)
    end

    expect(missing).to be_empty
  end
end
