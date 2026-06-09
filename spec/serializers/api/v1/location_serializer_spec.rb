# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::LocationSerializer do
  it 'serialises identity, description and an ISO8601 updated_at' do
    location = create(:location, name: 'Kitchen', description: 'Top shelf')
    expect(described_class.new(location).as_json).to eq(
      id: location.id, name: 'Kitchen', description: 'Top shelf',
      updated_at: location.updated_at.iso8601
    )
  end
end
