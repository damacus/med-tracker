# frozen_string_literal: true

class NotificationPreference < ApplicationRecord
  has_paper_trail

  belongs_to :household, optional: true
  belongs_to :person

  before_validation :assign_household

  PERIODS = %i[morning afternoon evening night].freeze

  def time_for_period(period)
    send(:"#{period}_time")
  end

  private

  def assign_household
    self.household ||= person&.household || Current.household
  end
end
