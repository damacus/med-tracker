# frozen_string_literal: true

require 'test_helper'

class ApplicationSystemTestCase < ActionDispatch::SystemTestCase
  driven_by :playwright, using: :headless_chrome, screen_size: [1400, 1400]
end
