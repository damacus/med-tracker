# frozen_string_literal: true

require 'json'
require 'rails_helper'

RSpec.describe JSON do
  let(:config) { described_class.parse(Rails.root.join('renovate.json').read) }

  it 'automerge dependency updates only after the complete branch is green' do
    expect(config).to include(
      'automerge' => true,
      'automergeType' => 'pr',
      'ignoreTests' => false,
      'platformAutomerge' => false
    )
  end

  it 'automerge lockfile maintenance only after the complete branch is green' do
    expect(config.fetch('lockFileMaintenance')).to include(
      'enabled' => true,
      'automerge' => true,
      'automergeType' => 'pr',
      'ignoreTests' => false
    )
  end
end
