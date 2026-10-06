# frozen_string_literal: true

class AuditLedgerEntry < ApplicationRecord
  belongs_to :household, optional: true

  validates :chain_key, :chain_epoch, :sequence, :source_table, :source_id, :entry_hash,
            :hash_algorithm, :schema_version, :retention_policy_version, :retain_until,
            :occurred_at, presence: true

  def readonly?
    persisted?
  end
end
