# frozen_string_literal: true

require 'rails_helper'
require 'erb'

module SharedDatabaseLoginCompatibility
end

RSpec.describe SharedDatabaseLoginCompatibility do
  let(:compose_config) do
    YAML.safe_load(Rails.root.join('compose.yaml').read, aliases: true)
  end
  let(:local_taskfile) do
    YAML.safe_load(Rails.root.join('Taskfiles/local.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  it 'uses the existing shared login for local tasks' do
    shared_url = 'postgres://{{.DB_USER}}:{{.DB_PASSWORD}}@localhost:{{.DB_PORT}}/{{.DB_NAME}}'

    expect(local_taskfile.dig('vars', 'DATABASE_URL')).to eq(shared_url)
    %w[db:prepare test test:browser test:all].each do |task_name|
      expect(local_taskfile.dig('tasks', task_name, 'env')).to include('DATABASE_URL' => '{{.DATABASE_URL}}')
      expect(local_taskfile.dig('tasks', task_name, 'env')).not_to include('DATABASE_ROLE')
    end
  end

  it 'uses shared Compose credentials while migrations complete before web starts' do
    expect_compose_login('dev', 'medtracker', 'medtracker_queue')
    expect_compose_login('test', 'medtracker')
    expect_compose_login(
      'prod', 'medtracker',
      'medtracker_production_queue', 'medtracker_production_cache', 'medtracker_production_cable'
    )

    %w[dev test prod].each do |environment|
      expect(compose_config.dig('services', "migrate-#{environment}", 'environment', 'DATABASE_ROLE'))
        .to match(/:-}\z/)
      expect(compose_config.dig('services', "web-#{environment}", 'depends_on', "migrate-#{environment}", 'condition'))
        .to eq('service_completed_successfully')
    end
  end

  it 'derives auxiliary database URLs from the shared production login' do
    with_database_environment('DATABASE_URL' => 'postgresql://med-tracker:password@database/medtracker') do
      config = YAML.safe_load(ERB.new(Rails.root.join('config/database.yml').read).result, aliases: true)

      expect(config.dig('development', 'queue', 'url'))
        .to eq('postgresql://med-tracker:password@database/medtracker_queue')
      expect(config.dig('production', 'queue', 'url'))
        .to eq('postgresql://med-tracker:password@database/medtracker_production_queue')
      expect(config.dig('production', 'cache', 'url'))
        .to eq('postgresql://med-tracker:password@database/medtracker_production_cache')
      expect(config.dig('production', 'cable', 'url'))
        .to eq('postgresql://med-tracker:password@database/medtracker_production_cable')
    end
  end

  def expect_compose_login(environment, login, *auxiliary_databases)
    services = %W[migrate-#{environment} web-#{environment}]

    services.each do |service|
      expect(compose_config.dig('services', service, 'environment', 'DATABASE_URL')).to include("://#{login}:")
    end
    auxiliary_databases.each do |database|
      expect(compose_config.dig('services', "migrate-#{environment}", 'environment').values)
        .to include(a_string_including("://#{login}:", "/#{database}"))
    end
  end

  def with_database_environment(environment)
    keys = environment.keys + %w[
      DATABASE_ROLE
      SOLID_QUEUE_DATABASE_URL
      SOLID_CACHE_DATABASE_URL
      SOLID_CABLE_DATABASE_URL
    ]
    original_environment = ENV.to_h.slice(*keys)
    keys.each { |key| ENV.delete(key) }
    ENV.update(environment)
    yield
  ensure
    keys.each { |key| original_environment.key?(key) ? ENV[key] = original_environment[key] : ENV.delete(key) }
  end
end
