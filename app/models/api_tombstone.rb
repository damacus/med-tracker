# frozen_string_literal: true

class ApiTombstone < ApplicationRecord
  belongs_to :household
  belongs_to :account
  belongs_to :household_membership

  validates :record_type, :record_portable_id, :action, :deleted_at, presence: true
end
