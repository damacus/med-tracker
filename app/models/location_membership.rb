# frozen_string_literal: true

class LocationMembership < ApplicationRecord
  has_paper_trail

  belongs_to :location
  belongs_to :person

  validates :person_id, uniqueness: {scope: :location_id}
end
