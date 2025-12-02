# frozen_string_literal: true

require 'capybara/rspec'

# Performance-optimized browser args for CI and local testing
PLAYWRIGHT_BROWSER_ARGS = [
  '--disable-blink-features=AutomationControlled',
  '--disable-features=PasswordManager',
  '--disable-save-password-bubble',
  '--disable-background-timer-throttling',
  '--disable-backgrounding-occluded-windows',
  '--disable-renderer-backgrounding',
  '--disable-dev-shm-usage',
  '--no-sandbox'
].freeze

Capybara.register_driver :playwright do |app|
  Capybara::Playwright::Driver.new(
    app,
    browser_type: :chromium,
    headless: (ENV.fetch('CI', nil) || ENV.fetch('PLAYWRIGHT_HEADLESS', nil)).present?,
    args: PLAYWRIGHT_BROWSER_ARGS
  )
end

RSpec.configure do |config|
  config.before(:each, type: :system) do
    driven_by :playwright, using: :chromium, screen_size: [1400, 1400], options: {
      args: PLAYWRIGHT_BROWSER_ARGS
    }
  end

  # Disable CSS animations and transitions for faster tests
  config.before(:each, type: :system) do
    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(script: <<~JS)
        const style = document.createElement('style');
        style.textContent = `
          *, *::before, *::after {
            transition-duration: 0s !important;
            transition-delay: 0s !important;
            animation-duration: 0s !important;
            animation-delay: 0s !important;
          }
        `;
        document.head.appendChild(style);
      JS
    end
  rescue StandardError
    # Ignore if page not ready yet
  end
end
