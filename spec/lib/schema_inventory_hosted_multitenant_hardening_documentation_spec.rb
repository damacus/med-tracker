# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SchemaInventory do
  let(:audit_path) { Rails.root.join('docs/security/hosted-multi-tenant-hardening-audit.md') }
  let(:runbook_path) { Rails.root.join('docs/operations/hosted-private-beta-runbook.md') }
  let(:audit_doc) { audit_path.read }
  let(:runbook_doc) { runbook_path.read }

  it 'maps every hosted hardening requirement to current evidence, gaps, tests, and beta status' do
    expected_requirements.each do |requirement|
      expect(audit_doc).to include("| #{requirement} |")
    end

    expect(audit_doc).to include(
      '| Requirement | Current evidence | Gap / decision | Severity | Owner issue | Tests required | Beta status |'
    )
    expect(audit_doc).to include('NO-GO')
    expect(audit_doc).to include('Platform admin')
    expect(audit_doc).to include('export + purge')
  end

  it 'keeps every requirement row classified with evidence, gaps, owner issue, tests, and beta status',
     :aggregate_failures do
    requirement_rows.each do |requirement, columns|
      expect(columns.fetch(:evidence)).to be_present, "#{requirement} evidence is blank"
      expect(columns.fetch(:gap)).to be_present, "#{requirement} gap is blank"
      expect(columns.fetch(:severity)).to be_present, "#{requirement} severity is blank"
      expect(columns.fetch(:owner_issue)).to match(/\AHMT-\d{3}\z/), "#{requirement} owner issue is invalid"
      expect(columns.fetch(:tests)).to be_present, "#{requirement} tests are blank"
      if tenant_foundation_requirement?(requirement)
        expect(columns.fetch(:gap)).to eq('Closed.'), "#{requirement} gap must be closed"
        expect(columns.fetch(:beta_status)).to eq('GO'), "#{requirement} beta status must be GO"
      else
        expect(columns.fetch(:beta_status)).to eq('NO-GO'), "#{requirement} beta status must remain NO-GO"
      end
    end
  end

  it 'records partial evidence for the current FR7-FR10 hosted hardening slices' do
    expect(requirement_rows.fetch('FR7').fetch(:evidence)).to include('Admin::MembershipRolesController')
    expect(requirement_rows.fetch('FR8').fetch(:evidence)).to include('Platform::SupportAccessSessionsController')
    expect(requirement_rows.fetch('FR9').fetch(:evidence)).to include('invitation email')
    expect(requirement_rows.fetch('FR10').fetch(:evidence)).to include('HostedPrivilegedActionMfa')
  end

  it 'documents hosted beta onboarding, support access, export/offboarding, and restore checks', :aggregate_failures do
    expect(runbook_doc).to include('Hosted Private Beta Runbook')
    expect(runbook_doc).to include('Onboarding')
    expect(runbook_doc).to include('Support access')
    expect(runbook_doc).to include('Export and purge')
    expect(runbook_doc).to include('Restore test')
    expect(runbook_doc).to include('DATABASE_ROLE=med_tracker_app')
  end

  def expected_requirements
    (1..18).map { |number| "FR#{number}" } + (1..4).map { |number| "NFR#{number}" }
  end

  def tenant_foundation_requirement?(requirement)
    %w[FR1 FR2 FR3 FR4].include?(requirement)
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
