# frozen_string_literal: true

ENV['RAILS_ENV'] ||= 'test'
require_relative '../config/environment'
require 'rails/test_help'
require 'minitest/reporters'
require 'minitest/minitest_reporter_plugin'
require_relative 'test_helpers/sign_in_helper'

Minitest::Reporters.use!(
  Minitest::Reporters::DefaultReporter.new(
    color: true,
    slow_count: 5,
    slow_suite_count: 3,
    verbose: ENV.fetch('VERBOSE', false)
  )
)
Minitest.extensions << 'minitest_reporter' unless Minitest.extensions.include?('minitest_reporter')

module ActiveSupport
  class TestCase
    # Parallelization disabled due to upstream bugs in Ruby 4.0.1 + pg 1.6.3:
    #   - Forks: pg native C connect_start segfaults after fork
    #   - Threads: ActiveRecord connection pool uses NullLock, causing
    #     connection ownership errors across threads
    # Re-enable when pg gem or Rails 8.1 ships fixes.
    parallelize(workers: 1)

    # Setup all fixtures in test/fixtures/*.yml for all tests in alphabetical order.
    self.fixture_paths = [Rails.root.join('spec/fixtures')]
    fixtures :all

    # Add more helper methods to be used by all tests here...
  end
end

module ActionDispatch
  class IntegrationTest
    include SignInHelper
  end
end
