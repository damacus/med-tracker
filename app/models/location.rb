# frozen_string_literal: true

class Location < ApplicationRecord
  has_paper_trail

  belongs_to :household, optional: true
  has_many :medications, dependent: :destroy
  has_many :location_memberships, dependent: :destroy
  has_many :members, through: :location_memberships, source: :person

  validates :name, presence: true
  validates :name, uniqueness: { scope: :household_id, case_sensitive: false }, if: :household_id?
end
