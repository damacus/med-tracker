# frozen_string_literal: true

class NotificationPreference < ApplicationRecord
  belongs_to :person

  PERIODS = %i[morning afternoon evening night].freeze

  def time_for_period(period)
    send(:"#{period}_time")
  end
end
