# frozen_string_literal: true

class AuditCheckpoint < ApplicationRecord
  belongs_to :household, optional: true
  belongs_to :audit_signing_key, optional: true
  has_one :audit_export_delivery, dependent: :restrict_with_error

  validates :chain_key, :chain_epoch, :checkpoint_kind, :sequence, :entry_hash, presence: true
end
