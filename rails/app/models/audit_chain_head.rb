# frozen_string_literal: true

class AuditChainHead < ApplicationRecord
  belongs_to :household, optional: true

  validates :chain_key, :chain_epoch, :epoch_kind, :last_sequence, presence: true
end
