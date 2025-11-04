# frozen_string_literal: true

require 'rails_helper'

# System tests need special database handling because JavaScript runs in a separate thread
# and can't see transactional fixtures. We use DatabaseCleaner with truncation strategy.
RSpec.configure do |config|
  config.use_transactional_fixtures = false

  config.before(:suite) do
    # For SQLite, disable foreign key constraints during cleaning
    if ActiveRecord::Base.connection.adapter_name.downcase.include?('sqlite')
      ActiveRecord::Base.connection.execute('PRAGMA foreign_keys = OFF')
      DatabaseCleaner.clean_with(:truncation)
      ActiveRecord::Base.connection.execute('PRAGMA foreign_keys = ON')
    else
      DatabaseCleaner.clean_with(:truncation)
    end
  end

  config.before do |example|
    DatabaseCleaner.strategy = example.metadata[:js] ? :truncation : :transaction
    DatabaseCleaner.start
  end

  config.after do
    DatabaseCleaner.clean
  end
end
