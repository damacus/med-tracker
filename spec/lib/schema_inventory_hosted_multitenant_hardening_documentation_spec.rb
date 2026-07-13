# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SchemaInventory do
  let(:audit_path) { Rails.root.join('docs/security/hosted-multi-tenant-hardening-audit.md') }
  let(:runbook_path) { Rails.root.join('docs/operations/hosted-private-beta-runbook.md') }
  let(:audit_doc) { audit_path.read }
  let(:runbook_doc) { runbook_path.read }

  it 'maps every hosted hardening requirement to current evidence, gaps, tests, and beta status', :aggregate_failures do
    expected_requirements.each do |requirement|
      expect(audit_doc).to include("| #{requirement} |")
    end

    expect(audit_doc).to include(
      '| Requirement | Current evidence | Gap / decision | Severity | Owner issue | Tests required | Beta status |'
    )
    expect(audit_doc).to include('Date: 2026-07-13')
    expect(audit_doc).to include('**Current status: NO-GO**')
    expect(audit_doc).to include('Platform admin')
    expect(audit_doc).to include('export + purge')
    expect(audit_doc).to include('## Review Process')
    expect(audit_doc).not_to match(/HMT-\d{3}/)
  end

  it 'keeps every requirement row classified with evidence, gaps, owner issue, tests, and beta status',
     :aggregate_failures do
    requirement_rows.each do |requirement, columns|
      expect(columns.fetch(:evidence)).to be_present, "#{requirement} evidence is blank"
      expect(columns.fetch(:gap)).to be_present, "#{requirement} gap is blank"
      expect(columns.fetch(:severity)).to be_present, "#{requirement} severity is blank"
      expect(columns.fetch(:tests)).to be_present, "#{requirement} tests are blank"
      if columns.fetch(:beta_status) == 'GO'
        expect(columns.fetch(:gap)).to start_with('Closed:'), "#{requirement} closure decision is missing"
        expect(columns.fetch(:owner_issue)).to eq('Closed'), "#{requirement} must not retain an open owner"
      else
        expect(columns.fetch(:beta_status)).to eq('NO-GO'), "#{requirement} beta status must remain NO-GO"
        expect(columns.fetch(:gap)).to include('Launch impact:'), "#{requirement} launch impact is missing"
        expect(columns.fetch(:owner_issue)).to match(github_issue_link), "#{requirement} owner issue is invalid"
      end
    end
  end

  it 'classifies the controls against current main without overstating the hosted launch decision' do
    expect(go_requirements).to contain_exactly(
      'FR1', 'FR2', 'FR3', 'FR4', 'FR5', 'FR6', 'FR10', 'FR12', 'FR13', 'FR17', 'FR18', 'NFR1', 'NFR2'
    )
    expect(no_go_requirements).to contain_exactly(
      'FR7', 'FR8', 'FR9', 'FR11', 'FR14', 'FR15', 'FR16', 'NFR3', 'NFR4'
    )
  end

  it 'documents hosted beta onboarding, support access, export/offboarding, and restore checks', :aggregate_failures do
    expect(runbook_doc).to include('Hosted Private Beta Runbook')
    expect(runbook_doc).to include('Onboarding')
    expect(runbook_doc).to include('Support access')
    expect(runbook_doc).to include('Export and purge')
    expect(runbook_doc).to include('Restore test')
    expect(runbook_doc).to include('DATABASE_ROLE=med_tracker_app')
  end

  it 'publishes the pre-0.5 database upgrade runbook from the docs home page', :aggregate_failures do
    docs_home = Rails.root.join('docs/index.md').read
    upgrade_doc = Rails.root.join('docs/pre-0-5-database-upgrade.md').read

    expect(docs_home).to include('pre-0-5-database-upgrade.md')
    expect(upgrade_doc).to include('Pre-0.5 database upgrade')
    expect(upgrade_doc).to include('DATABASE_ROLE=med_tracker_owner')
    expect(upgrade_doc).to include('med_tracker:pre_0_5_database_upgrade_preflight')
    expect(upgrade_doc).to include('med_tracker_owner')
    expect(upgrade_doc).to include('med_tracker_app')
    expect(upgrade_doc).to include('BYPASSRLS')
  end

  def expected_requirements
    (1..18).map { |number| "FR#{number}" } + (1..4).map { |number| "NFR#{number}" }
  end

  def github_issue_link
    %r{\[#\d+\]\(https://github\.com/damacus/med-tracker/issues/\d+\)}
  end

  def go_requirements
    requirements_with_status('GO')
  end

  def no_go_requirements
    requirements_with_status('NO-GO')
  end

  def requirements_with_status(status)
    requirement_rows.filter_map { |requirement, columns| requirement if columns.fetch(:beta_status) == status }
  end

  def requirement_rows
    audit_doc.lines.filter_map do |line|
      cells = line.strip.split('|').map(&:strip)
      requirement = cells[1]
      next unless expected_requirements.include?(requirement)

      [requirement, requirement_row_columns(cells)]
    end.to_h
  end

  def requirement_row_columns(cells)
    {
      evidence: cells[2],
      gap: cells[3],
      severity: cells[4],
      owner_issue: cells[5],
      tests: cells[6],
      beta_status: cells[7]
    }
  end
end
