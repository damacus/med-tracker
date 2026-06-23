# frozen_string_literal: true

require 'json'
require 'rails_helper'

RSpec.describe JSON do
  let(:config) { described_class.parse(Rails.root.join('renovate.json').read) }
  let(:package_rules) { config.fetch('packageRules') }

  def rule_for(description)
    package_rules.find { |rule| rule.fetch('description') == description }
  end

  it 'does not automerge every dependency update from the shared preset' do
    expect(config.fetch('automerge')).to be(false)
  end

  it 'requires review for GitHub Actions non-major updates' do
    rule = rule_for('Require review for GitHub Actions non-major updates')

    expect(rule.fetch('matchManagers')).to eq(['github-actions'])
    expect(rule.fetch('matchUpdateTypes')).to contain_exactly('minor', 'patch', 'digest')
    expect(rule.fetch('automerge')).to be(false)
    expect(rule.fetch('ignoreTests')).to be(false)
  end

  it 'requires review for npm development dependency non-major updates' do
    rule = rule_for('Require review for npm development dependency non-major updates')

    expect(rule.fetch('matchManagers')).to eq(['npm'])
    expect(rule.fetch('matchDepTypes')).to eq(['devDependencies'])
    expect(rule.fetch('matchUpdateTypes')).to contain_exactly('minor', 'patch')
    expect(rule.fetch('automerge')).to be(false)
  end

  it 'requires review for Bundler patch updates' do
    rule = rule_for('Require review for Bundler patch updates')

    expect(rule.fetch('matchManagers')).to eq(['bundler'])
    expect(rule.fetch('matchUpdateTypes')).to eq(['patch'])
    expect(rule.fetch('automerge')).to be(false)
  end

  it 'requires review for Docker patch and digest updates' do
    rule = rule_for('Require review for Docker patch and digest updates')

    expect(rule.fetch('matchManagers')).to contain_exactly('dockerfile', 'docker-compose')
    expect(rule.fetch('matchUpdateTypes')).to contain_exactly('patch', 'digest')
    expect(rule.fetch('automerge')).to be(false)
  end

  it 'requires review for lockfile maintenance' do
    lockfile_maintenance = config.fetch('lockFileMaintenance')

    expect(lockfile_maintenance.fetch('enabled')).to be(true)
    expect(lockfile_maintenance.fetch('automerge')).to be(false)
    expect(lockfile_maintenance.fetch('ignoreTests')).to be(false)
  end

  it 'keeps major updates manual' do
    rule = rule_for('Keep major updates manual')

    expect(rule.fetch('matchUpdateTypes')).to eq(['major'])
    expect(rule.fetch('automerge')).to be(false)
  end
end
