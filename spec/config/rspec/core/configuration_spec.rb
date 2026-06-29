# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RSpec::Core::Configuration do
  it 'loads household fixtures globally for tenant-owned fixture records' do
    expect(Array(RSpec.configuration.global_fixtures)).to include(:households)
  end
end
