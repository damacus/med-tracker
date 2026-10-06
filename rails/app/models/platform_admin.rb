# frozen_string_literal: true

class PlatformAdmin < ApplicationRecord
  belongs_to :account

  has_many :support_access_sessions, dependent: :destroy

  enum :status, { active: 'active', disabled: 'disabled' }, validate: true

  validates :account_id, uniqueness: true
end
