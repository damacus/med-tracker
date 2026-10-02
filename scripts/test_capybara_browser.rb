require 'capybara/playwright'

app = ->(_env) { [200, { 'content-type' => 'text/plain' }, ['ok']] }
driver = Capybara::Playwright::Driver.new(app, browser_type: :chromium, headless: true)
driver.send(:browser)
driver.send(:quit)
puts 'Capybara Playwright launch preflight passed'
