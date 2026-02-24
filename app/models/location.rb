# frozen_string_literal: true

class Location < ApplicationRecord
  has_many :medicines, dependent: :destroy
  has_many :location_memberships, dependent: :destroy
  has_many :members, through: :location_memberships, source: :person

  validates :name, presence: true, uniqueness: { case_sensitive: false }
end
