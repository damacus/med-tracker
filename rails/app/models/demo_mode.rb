# frozen_string_literal: true

module DemoMode
  RESET_SCHEDULE = 'Every Sunday at 04:15 Europe/London'

  module_function

  def enabled?
    ENV['DEMO_MODE'] == 'true'
  end

  def reset_schedule
    RESET_SCHEDULE
  end
end
