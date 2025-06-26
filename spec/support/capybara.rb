# frozen_string_literal: true

require 'capybara/rspec'

Capybara.register_driver :playwright_driver do |app|
  Capybara::Playwright::Driver.new(app, browser: :chromium)
end

RSpec.configure do |config|
  config.before(:each, type: :system) do
    driven_by :playwright_driver
  end
end
