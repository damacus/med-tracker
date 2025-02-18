require "test_helper"
require "capybara/rails"
require "capybara-playwright-driver"

class ApplicationSystemTestCase < ActionDispatch::SystemTestCase
  include ActiveSupport::Testing::TimeHelpers
  driven_by :playwright, options: {
    browser: :chromium,
    headless: true,
    screen_size: [ 1400, 1400 ]
  }

  def setup
    super
    Capybara.default_max_wait_time = 5
  end

  def teardown
    super
  end
end
