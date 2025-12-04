# frozen_string_literal: true

# Utility to load RSpec fixtures directly into the database outside the test framework helpers.
class SpecFixtureLoader
  FIXTURES_PATH = Rails.root.join('spec/fixtures')

  class << self
    def load(*fixture_names)
      new(fixture_names.flatten.map(&:to_s)).load
    end
  end

  def initialize(fixture_names)
    @fixture_names = fixture_names
  end

  def load
    ActiveRecord::FixtureSet.reset_cache
    # Load fixtures - foreign keys are deferrable so order doesn't matter within a transaction
    ActiveRecord::FixtureSet.create_fixtures(FIXTURES_PATH, fixture_names)
  end

  private

  attr_reader :fixture_names
end
