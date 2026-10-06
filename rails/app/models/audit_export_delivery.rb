# frozen_string_literal: true

class AuditExportDelivery < ApplicationRecord
  belongs_to :audit_ledger_entry, optional: true
  belongs_to :audit_checkpoint, optional: true

  enum :status, { pending: 'pending', delivered: 'delivered', failed: 'failed' }, validate: true

  validates :status, :attempts, presence: true
  validate :exactly_one_export_record

  def export_record
    audit_ledger_entry || audit_checkpoint
  end

  private

  def exactly_one_export_record
    return if [audit_ledger_entry, audit_checkpoint].one?(&:present?)

    errors.add(:base, 'must reference exactly one audit record')
  end
end
