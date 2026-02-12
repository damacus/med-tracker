# frozen_string_literal: true

ENV['RAILS_ENV'] ||= 'test'
require_relative '../config/environment'
require 'rails/test_help'
require_relative 'test_helpers/sign_in_helper'

module ActiveSupport
  class TestCase
    # Parallelization disabled: suite is small and parallel forks crash
    # with heavy fixture/DB usage. Re-enable when suite grows large enough.
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
