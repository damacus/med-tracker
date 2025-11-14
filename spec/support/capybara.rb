# frozen_string_literal: true

require 'capybara/rspec'

Capybara.register_driver :playwright do |app|
  Capybara::Playwright::Driver.new(
    app,
    browser: :chromium,
    browser_options: {
      args: [
        '--disable-blink-features=AutomationControlled',
        '--disable-features=PasswordManager',
        '--disable-save-password-bubble'
      ]
    }
  )
end

RSpec.configure do |config|
  config.before(:each, type: :system) do
    driven_by :playwright, using: :chromium, screen_size: [1400, 1400], options: {
      args: [
        '--disable-blink-features=AutomationControlled',
        '--disable-features=PasswordManager',
        '--disable-save-password-bubble'
      ]
    }
  end
end
