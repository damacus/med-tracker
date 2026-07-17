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
      notification_events
      carer_relationships
      household_memberships
      person_access_grants
      household_invitations
      household_invitation_grants
      medication_review_prompts
      security_audit_events
      active_storage_attachments
    ]
  end
  let(:expected_global_tables) do
    %w[
      api_app_tokens
      api_household_selection_grants
      api_sessions
      barcode_catalog_entries
      households
      medication_review_evidence_records
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

  it 'keeps every household-owned table in the fresh-schema RLS installer' do
    schema_source = Rails.root.join('db/schema.rb').read
    rls_tables = schema_source.match(/%w\[(?<tables>.*?)\]\.each do \|table_name\|/m)[:tables].split

    expect(rls_tables).to match_array(described_class.household_owned_tables)
  end

  it 'loads the fresh schema before runtime roles have been bootstrapped' do
    schema_source = Rails.root.join('db/schema.rb').read
    invitation_grant = schema_source.match(/DO \$role_grant\$(?<body>.*?)\$role_grant\$;/m)[:body]

    expect(invitation_grant).to include(
      "IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app')",
      'GRANT EXECUTE ON FUNCTION med_tracker.current_invitation_token_digest() TO med_tracker_app;'
    )
  end

  it 'keeps immutable audit tables tenant-classified while excluding them from destructive purge' do
    expect(described_class.immutable_audit_tables).to contain_exactly('security_audit_events', 'versions')
    expect(described_class.household_owned_tables).to include('security_audit_events')
    expect(described_class.global_tables).to include('versions')
    expect(described_class.purgeable_household_owned_tables)
      .to match_array(described_class.household_owned_tables - described_class.immutable_audit_tables)
  end

  context 'with hosted household lifecycle operations' do
    let(:taskfile) { Rails.root.join('Taskfile.yml').read }
    let(:runbook) { Rails.root.join('docs/operations/hosted-private-beta-runbook.md').read }

    it 'exposes exact task-wrapper commands and required operator inputs' do
      expect(taskfile).to include(
        'household-lifecycle:export:',
        'household-lifecycle:hold:',
        'household-lifecycle:release-hold:',
        'household-lifecycle:offboard:',
        'household-lifecycle:purge:'
      )
      expect(runbook).to include(
        'task household-lifecycle:export HOUSEHOLD_ID=',
        'task household-lifecycle:hold HOUSEHOLD_ID=',
        'task household-lifecycle:release-hold HOLD_ID=',
        'task household-lifecycle:offboard HOUSEHOLD_ID=',
        'task household-lifecycle:purge HOUSEHOLD_ID='
      )
    end

    it 'documents safe retries, hold refusal, configurable retention, and sanitized evidence' do
      expect(runbook).to include(
        'HOUSEHOLD_EXPORT_RETENTION_DAYS',
        'HOUSEHOLD_EXPORT_GENERATION_TIMEOUT_MINUTES',
        'safe to retry',
        'active retention hold',
        'failure_code',
        'last_completed_table',
        'Never retain free-text reasons, attachment contents, credentials, or health data in command output.'
      )
    end

    it 'keeps retention-hold reasons out of the rendered child command' do
      hold_task = taskfile.match(/  household-lifecycle:hold:\n(?<body>.*?)(?=\n  \S)/m)[:body]
      command = hold_task.match(/COMMAND: '(?<command>.*)'/)[:command]

      expect(command).not_to include('REASON', '.REASON')
      expect(hold_task).to include("DOCKER_RUN_ARGS: '-e REASON'", "REASON: '{{ .REASON }}'")
    end
  end
end
