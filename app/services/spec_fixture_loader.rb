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
    ActiveRecord::Base.connection.disable_referential_integrity do
      # Load all fixtures at once to avoid foreign key validation issues
      ActiveRecord::FixtureSet.create_fixtures(FIXTURES_PATH, fixture_names)
    end
  end

  private

  attr_reader :fixture_names
end
