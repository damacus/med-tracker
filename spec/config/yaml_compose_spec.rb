# frozen_string_literal: true

require 'rails_helper'

RSpec.describe YAML do
  let(:compose_config) do
    described_class.safe_load(Rails.root.join('compose.yaml').read, aliases: true)
  end

  it 'isolates public assets in development web container' do
    expect(compose_config.dig('services', 'web-dev', 'tmpfs')).to include('/app/public/assets:uid=1000,gid=1000')
  end

  it 'isolates public assets in test web container' do
    expect(compose_config.dig('services', 'web-test', 'tmpfs')).to include('/app/public/assets:uid=1000,gid=1000')
  end

  it 'mounts development PostgreSQL data at the PostgreSQL 18 data root' do
    expect(compose_config.dig('services', 'db-dev', 'volumes')).to include(
      'medtracker_dev_postgres:/var/lib/postgresql'
    )
  end

  it 'mounts production PostgreSQL data at the PostgreSQL 18 data root' do
    expect(compose_config.dig('services', 'db-prod', 'volumes')).to include(
      'medtracker_prod_postgres:/var/lib/postgresql'
    )
  end

  it 'builds the development migrate container from the development image target' do
    expect(compose_config.dig('services', 'migrate-dev', 'image')).to eq('med-tracker-web-dev')
    expect(compose_config.dig('services', 'migrate-dev', 'build', 'target')).to eq('assets')
    expect(compose_config.dig('services', 'migrate-dev', 'build', 'args', 'RAILS_ENV')).to eq('development')
  end

  it 'builds the test migrate container from the test image target' do
    expect(compose_config.dig('services', 'migrate-test', 'image')).to eq('med-tracker-web-test')
    expect(compose_config.dig('services', 'migrate-test', 'build', 'target')).to eq('test')
  end
end
