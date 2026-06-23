# frozen_string_literal: true

require 'rails_helper'
require 'erb'

module DatabaseRoleConfig
end

RSpec.describe DatabaseRoleConfig do
  around do |example|
    original_database_role = ENV.fetch('DATABASE_ROLE', nil)
    ENV['DATABASE_ROLE'] = 'med_tracker_app'
    example.run
  ensure
    ENV['DATABASE_ROLE'] = original_database_role
  end

  it 'applies DATABASE_ROLE to primary tenant database connections' do
    expect(config.dig('development', 'primary', 'variables', 'role')).to eq('med_tracker_app')
    expect(config.dig('test', 'variables', 'role')).to eq('med_tracker_app')
    expect(config.dig('production', 'primary', 'variables', 'role')).to eq('med_tracker_app')
  end

  it 'does not apply DATABASE_ROLE to development auxiliary databases' do
    expect(config.dig('development', 'queue', 'variables')).to be_nil
  end

  it 'does not apply DATABASE_ROLE to production auxiliary databases' do
    expect(config.dig('production', 'queue', 'variables')).to be_nil
    expect(config.dig('production', 'cache', 'variables')).to be_nil
    expect(config.dig('production', 'cable', 'variables')).to be_nil
  end

  def config
    YAML.safe_load(ERB.new(Rails.root.join('config/database.yml').read).result, aliases: true)
  end
end
