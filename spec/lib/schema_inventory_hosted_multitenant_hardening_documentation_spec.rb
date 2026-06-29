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
end
