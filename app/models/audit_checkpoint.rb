# frozen_string_literal: true

class AuditCheckpoint < ApplicationRecord
  belongs_to :household, optional: true
  belongs_to :audit_signing_key, optional: true

  validates :chain_key, :chain_epoch, :checkpoint_kind, :sequence, :entry_hash, presence: true
end
