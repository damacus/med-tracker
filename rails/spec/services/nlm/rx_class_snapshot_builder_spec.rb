# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Nlm::RxClassSnapshotBuilder do
  it 'builds terminology for foundation and targeted medicines' do
    manifest = instance_double(OpenFda::SnapshotManifest, version: 2, all_selection: %w[warfarin ketorolac])
    client = instance_double(Nlm::RxClassClient, entries_for: [])

    snapshot = described_class.new(client: client, manifest: manifest, generated_on: Date.new(2026, 7, 10)).call

    expect(snapshot).to include('selection_version' => 2, 'generated_on' => '2026-07-10')
    expect(client).to have_received(:entries_for).with(%w[warfarin ketorolac])
  end
end
