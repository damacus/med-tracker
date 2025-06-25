# frozen_string_literal: true

require 'capybara/rspec'
require 'selenium-webdriver'

# This file configures Capybara to use Selenium with a headless Chrome browser
# for all system tests, ensuring a consistent and non-visual testing environment.

# Register the custom headless Chrome driver.
Capybara.register_driver :selenium_chrome_headless do |app|
  # Configure Chrome options to run in headless mode.
  options = Selenium::WebDriver::Chrome::Options.new
  options.add_argument('--headless')
  options.add_argument('--no-sandbox')
  options.add_argument('--disable-dev-shm-usage')
  options.add_argument('--window-size=1400,1400')

  # Create a new Selenium driver instance with these options.
  Capybara::Selenium::Driver.new(app, browser: :chrome, options: options)
end

# Configure RSpec to use our headless driver for system tests.
RSpec.configure do |config|
  config.before(:each, type: :system) do
    # Set the driver to our custom headless configuration.
    driven_by :selenium_chrome_headless
  end
end
