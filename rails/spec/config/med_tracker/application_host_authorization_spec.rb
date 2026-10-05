# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedTracker::Application do
  let(:production_config) { Rails.root.join('config/environments/production.rb').read }

  it 'requires APP_URL and derives production host authorization from it' do
    expect(production_config).to include("app_url = URI.parse(ENV.fetch('APP_URL'))")
    expect(production_config).to include("ENV.fetch('RAILS_ALLOWED_HOSTS', '')")
    expect(production_config).to include('config.hosts = allowed_hosts')
  end

  it 'keeps only health endpoints excluded from host authorization' do
    expect(production_config).to include("request.path == '/up' || request.path == '/health'")
  end
end
