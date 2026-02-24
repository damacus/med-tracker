# frozen_string_literal: true

class LocationMembership < ApplicationRecord
  belongs_to :location
  belongs_to :person

  validates :person_id, uniqueness: { scope: :location_id }
end
