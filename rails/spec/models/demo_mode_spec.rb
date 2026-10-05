# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoMode do
  around do |example|
    original_value = ENV.fetch('DEMO_MODE', nil)
    example.run
  ensure
    original_value.nil? ? ENV.delete('DEMO_MODE') : ENV['DEMO_MODE'] = original_value
  end

  it 'defaults to disabled' do
    ENV.delete('DEMO_MODE')

    expect(described_class).not_to be_enabled
  end

  it 'is enabled only by the explicit true value' do
    %w[1 yes TRUE false].each do |value|
      ENV['DEMO_MODE'] = value
      expect(described_class).not_to be_enabled
    end

    ENV['DEMO_MODE'] = 'true'

    expect(described_class).to be_enabled
  end

  it 'describes the externally scheduled reset' do
    expect(described_class.reset_schedule).to eq('Every Sunday at 04:15 Europe/London')
  end
end
