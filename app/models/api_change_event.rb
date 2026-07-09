# frozen_string_literal: true

class ApiChangeEvent < ApplicationRecord
  belongs_to :household
  belongs_to :account
  belongs_to :household_membership

  validates :record_type, :record_id, :action, :occurred_at, presence: true
end
