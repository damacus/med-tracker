# frozen_string_literal: true

require 'capybara/rspec'

Capybara.register_driver :playwright do |app|
  Capybara::Playwright::Driver.new(app, browser: :chromium)
end

RSpec.configure do |config|
  config.before(:each, type: :system) do
    driven_by :playwright, using: :chromium, screen_size: [1400, 1400]
  end
end
