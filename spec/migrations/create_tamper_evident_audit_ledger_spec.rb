# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260709131000_create_tamper_evident_audit_ledger')

RSpec.describe CreateTamperEvidentAuditLedger do
  it 'backfills existing evidence into a labelled legacy epoch and rotates to a live head' do
    with_rebuilt_ledger do |legacy_id|
      entry = AuditLedgerEntry.find_by!(source_table: 'versions', source_id: legacy_id)
      expect(entry.epoch_kind).to eq('legacy-baseline')
      expect_legacy_checkpoint(entry)
      expect(AuditChainHead.find_by!(chain_key: 'global')).to have_attributes(
        epoch_kind: 'live', last_sequence: 0, last_hash: nil
      )
    end
  end

  def with_rebuilt_ledger
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Migration.suppress_messages { described_class.new.down }
      legacy_id = insert_legacy_version
      ActiveRecord::Migration.suppress_messages { described_class.new.up }
      yield legacy_id
      raise ActiveRecord::Rollback
    end
  end

  def insert_legacy_version
    ActiveRecord::Base.connection.select_value(<<~SQL.squish)
      INSERT INTO versions (item_type, item_id, event, object, created_at, audit_context)
      VALUES ('LegacyAudit', 1, 'legacy.test', '{"outcome":"success"}', CURRENT_TIMESTAMP - INTERVAL '1 day', '{}')
      RETURNING id
    SQL
  end

  def expect_legacy_checkpoint(entry)
    checkpoint = AuditCheckpoint.find_by!(chain_epoch: entry.chain_epoch)
    final_entry = AuditLedgerEntry.where(chain_epoch: entry.chain_epoch).order(:sequence).last
    expect(checkpoint).to have_attributes(
      checkpoint_kind: 'legacy-baseline',
      sequence: final_entry.sequence,
      entry_hash: final_entry.entry_hash
    )
  end
end
