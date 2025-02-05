require "test_helper"

class PlaywrightSystemTest < ActionDispatch::SystemTestCase
  driven_by :selenium, using: :headless_chrome, screen_size: [1400, 1400]

  def setup
    super
    @page = Capybara.current_session.driver.browser
  end

  def visit(*args)
    Capybara.current_session.visit(*args)
    @page = Capybara.current_session.driver.browser
  end
end
