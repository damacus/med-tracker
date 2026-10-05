# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::SnapshotManifest do
  subject(:manifest) { described_class.new }

  it 'defines exactly 80 unique foundation medicine selections' do
    expect(manifest.selection.size).to eq(80)
    expect(manifest.selection.uniq.size).to eq(80)
  end

  it 'defines 40 additional medicines selected for explicit target mentions' do
    targeted_selection = manifest.targeted_selection

    expect(targeted_selection.size).to eq(40)
    expect(targeted_selection.pluck('term').uniq.size).to eq(40)
    expect(targeted_selection).to include(
      { 'term' => 'doxazosin', 'interaction_targets' => %w[ibuprofen acetaminophen] },
      { 'term' => 'pilocarpine', 'interaction_targets' => %w[ibuprofen] }
    )
  end

  it 'combines foundation and targeted medicines into one unique selection' do
    expect(manifest.all_selection.size).to eq(120)
    expect(manifest.all_selection.uniq.size).to eq(120)
  end

  it 'has a version and public source attribution' do
    expect(manifest.version).to eq(2)
    expect(manifest.source).to eq('openFDA drug label API')
  end
end
