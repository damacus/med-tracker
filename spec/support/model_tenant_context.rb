# frozen_string_literal: true

RSpec.configure do |config|
  config.before(:each, type: :model) do
    Current.household ||= Household.first if ActiveRecord::Base.connection.table_exists?(:households)
  end

  config.after(:each, type: :model) do
    Current.reset
  end
end
