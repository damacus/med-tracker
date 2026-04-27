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

  it 'automerges GitHub Actions non-major updates after checks pass' do
    rule = rule_for('Auto-merge GitHub Actions non-major updates after checks pass')

    expect(rule.fetch('matchManagers')).to eq(['github-actions'])
    expect(rule.fetch('matchUpdateTypes')).to contain_exactly('minor', 'patch', 'digest')
    expect(rule.fetch('automerge')).to be(true)
    expect(rule.fetch('ignoreTests')).to be(false)
  end

  it 'automerges npm development dependency non-major updates after checks pass' do
    rule = rule_for('Auto-merge npm development dependency non-major updates after checks pass')

    expect(rule.fetch('matchManagers')).to eq(['npm'])
    expect(rule.fetch('matchDepTypes')).to eq(['devDependencies'])
    expect(rule.fetch('matchUpdateTypes')).to contain_exactly('minor', 'patch')
    expect(rule.fetch('automerge')).to be(true)
  end

  it 'automerges Bundler patch updates after checks pass' do
    rule = rule_for('Auto-merge Bundler patch updates after checks pass')

    expect(rule.fetch('matchManagers')).to eq(['bundler'])
    expect(rule.fetch('matchUpdateTypes')).to eq(['patch'])
    expect(rule.fetch('automerge')).to be(true)
  end

  it 'automerges Docker patch and digest updates after checks pass' do
    rule = rule_for('Auto-merge Docker patch and digest updates after checks pass')

    expect(rule.fetch('matchManagers')).to contain_exactly('dockerfile', 'docker-compose')
    expect(rule.fetch('matchUpdateTypes')).to contain_exactly('patch', 'digest')
    expect(rule.fetch('automerge')).to be(true)
  end

  it 'automerges lockfile maintenance after checks pass' do
    lockfile_maintenance = config.fetch('lockFileMaintenance')

    expect(lockfile_maintenance.fetch('enabled')).to be(true)
    expect(lockfile_maintenance.fetch('automerge')).to be(true)
    expect(lockfile_maintenance.fetch('ignoreTests')).to be(false)
  end

  it 'keeps major updates manual' do
    rule = rule_for('Keep major updates manual')

    expect(rule.fetch('matchUpdateTypes')).to eq(['major'])
    expect(rule.fetch('automerge')).to be(false)
  end
end
