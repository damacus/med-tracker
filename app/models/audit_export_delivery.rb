# frozen_string_literal: true

class AuditExportDelivery < ApplicationRecord
  belongs_to :audit_ledger_entry

  enum :status, { pending: 'pending', delivered: 'delivered', failed: 'failed' }, validate: true

  validates :status, :attempts, presence: true
end
