# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::HealthChecker do
  it 'classifies a missing application URL as unavailable' do
    expect(described_class.new(application_url: nil).call).to be(false)
  end

  it 'classifies a relative application URL as unavailable' do
    expect(described_class.new(application_url: 'canary').call).to be(false)
  end
end
