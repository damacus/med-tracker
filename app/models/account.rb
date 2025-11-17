# frozen_string_literal: true

class Account < ApplicationRecord
  include Rodauth::Rails.model

  enum :status, { unverified: 1, verified: 2, closed: 3 }

  has_one :person, dependent: :nullify

  validates :email, presence: true, uniqueness: true
end
