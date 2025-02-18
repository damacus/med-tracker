require "test_helper"
require "capybara/rails"
require "capybara-playwright-driver"
require "database_cleaner-active_record"

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
    DatabaseCleaner.strategy = :truncation
    DatabaseCleaner.clean
  end

  def teardown
    super
  end
end
