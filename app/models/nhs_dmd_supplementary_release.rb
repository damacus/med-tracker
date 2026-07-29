# frozen_string_literal: true

class NhsDmdSupplementaryRelease < ApplicationRecord
  validates :released_on, presence: true, uniqueness: true

  def self.current = order(released_on: :desc).first
end
