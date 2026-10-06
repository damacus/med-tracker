# frozen_string_literal: true

class AuditSigningKey < ApplicationRecord
  has_many :audit_checkpoints, dependent: :restrict_with_error

  validates :key_id, :algorithm, :public_key, :active_from, presence: true
end
