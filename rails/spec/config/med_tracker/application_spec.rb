# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedTracker::Application do
  it 'configures Rails time helpers from the TZ environment variable' do
    expect(Rails.application.config.time_zone).to eq(ENV.fetch('TZ', 'UTC'))
    expect(Rails.root.join('config/application.rb').read).to include("config.time_zone = ENV.fetch('TZ', 'UTC')")
  end
end
