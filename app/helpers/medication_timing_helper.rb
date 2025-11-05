# frozen_string_literal: true

module MedicationTimingHelper
  def time_until_available(next_time)
    return nil unless next_time

    diff = (next_time - Time.current).to_i
    return 'Available now' if diff <= 0

    hours = diff / 3600
    minutes = (diff % 3600) / 60

    if hours.zero?
      "Available in #{minutes}m"
    elsif minutes.zero?
      "Available in #{hours}h"
    else
      "Available in #{hours}h #{minutes}m"
    end
  end
end
