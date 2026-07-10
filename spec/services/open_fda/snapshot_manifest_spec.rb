# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::SnapshotManifest do
  subject(:manifest) { described_class.new }

  it 'defines exactly 80 unique medicine selections' do
    expect(manifest.selection.size).to eq(80)
    expect(manifest.selection.uniq.size).to eq(80)
  end

  it 'has a version and public source attribution' do
    expect(manifest.version).to eq(1)
    expect(manifest.source).to eq('openFDA drug label API')
  end
end
