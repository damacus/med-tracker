# frozen_string_literal: true

RSpec.configure do |config|
  config.before(
    file_path: %r{/spec/services/(global_search|global_search_query|medication_finder_search_responder)}
  ) do
    FixtureHouseholdSetup.apply!
  end

  config.around(
    file_path: %r{/spec/services/(global_search|global_search_query|medication_finder_search_responder)}
  ) do |example|
    household = Household.find_or_create_by!(slug: 'test-household') do |record|
      record.name = 'Test Household'
    end
    Current.household = household
    example.run
  ensure
    Current.reset
  end
end
