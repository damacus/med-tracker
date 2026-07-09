# frozen_string_literal: true

require 'rails_helper'

class ComplianceDocumentation
  REQUIRED_DOCS = {
    index: 'docs/compliance/index.md',
    intended_use: 'docs/compliance/intended-use.md',
    standards_boundary: 'docs/compliance/standards-boundary.md',
    evidence_map: 'docs/compliance/dcb0129/evidence-map.md',
    gap_analysis: 'docs/compliance/dcb0129/gap-analysis.md',
    clinical_risk_management_plan: 'docs/compliance/dcb0129/clinical-risk-management-plan.md',
    hazard_log: 'docs/compliance/dcb0129/hazard-log.md',
    clinical_safety_case: 'docs/compliance/dcb0129/clinical-safety-case.md',
    safety_incident_process: 'docs/compliance/dcb0129/safety-incident-process.md',
    learning_page: 'docs/compliance/learning/dcb0129-foundations.html'
  }.freeze

  def read(relative_path)
    Rails.root.join(relative_path).read
  end

  def exist?(relative_path)
    Rails.root.join(relative_path).exist?
  end

  def required_doc_paths
    REQUIRED_DOCS.values
  end

  def docs_home
    read('docs/index.md')
  end

  def nav_config
    read('zensical.toml')
  end
end

RSpec.describe ComplianceDocumentation do
  subject(:documentation) { described_class.new }

  it 'publishes every internal compliance foundation artifact' do
    documentation.required_doc_paths.each do |path|
      expect(documentation.exist?(path)).to be(true), "#{path} is missing"
    end
  end

  it 'keeps the intended-use boundary narrow and explicit', :aggregate_failures do
    intended_use = documentation.read('docs/compliance/intended-use.md')

    expect(intended_use).to include('medication tracking')
    expect(intended_use).to include('reminders')
    expect(intended_use).to include('rule-based safeguards')
    expect(intended_use).to include('audit history')
    expect(intended_use).to include('does not diagnose')
    expect(intended_use).to include('does not recommend treatment')
    expect(intended_use).to include('does not replace clinical judgement')
  end

  it 'explains the DCB0129, DCB0160, ISO 13485, and SaMD boundaries without overclaiming',
     :aggregate_failures do
    boundary = documentation.read('docs/compliance/standards-boundary.md')

    expect(boundary).to include('DCB0129 is the supplier/manufacturer side')
    expect(boundary).to include('DCB0160 is the deployment/use side')
    expect(boundary).to include('ISO 13485 is a medical-device quality management system standard')
    expect(boundary).to include('SaMD means software with its own medical purpose')
    expect(boundary).to include('deferred for this pass')
  end

  it 'maps DCB0129 foundation areas to evidence, gaps, owners, and tests', :aggregate_failures do
    evidence_map = documentation.read('docs/compliance/dcb0129/evidence-map.md')
    expected_rows = [
      'Intended use and scope',
      'Clinical governance',
      'Hazard identification',
      'Risk controls',
      'Verification evidence',
      'Release safety review',
      'Incident and near-miss process',
      'Post-release monitoring'
    ]

    expect(evidence_map).to include('| Area | Current evidence | Gap | Owner | Test or evidence link | Status |')
    expected_rows.each { |row| expect(evidence_map).to include("| #{row} |") }
  end

  it 'keeps compliance docs internally scoped and avoids approval/certification claims',
     :aggregate_failures do
    combined_docs = documentation.required_doc_paths.map { |path| documentation.read(path) }.join("\n")

    forbidden_claims = [
      'DCB0129 compliant',
      'DCB0160 compliant',
      'ISO 13485 compliant',
      'ISO 13485 certified',
      'NHS approved',
      'clinically approved',
      'MHRA registered',
      'UKCA marked'
    ]

    forbidden_claims.each do |claim|
      expect(combined_docs).not_to include(claim)
    end
  end

  it 'makes the learning material discoverable outside Markdown', :aggregate_failures do
    learning_page = documentation.read('docs/compliance/learning/dcb0129-foundations.html')

    expect(learning_page).to include('<!doctype html>')
    expect(learning_page).to include('DCB0129 first')
    expect(learning_page).to include('What we are not claiming')
    expect(learning_page).to include('Reading links')
    expect(learning_page).to include('Video starting points')
  end

  it 'links the compliance foundation from the docs home and Zensical navigation',
     :aggregate_failures do
    expect(documentation.docs_home).to include('compliance/index.md')
    expect(documentation.docs_home).to include('compliance/learning/dcb0129-foundations.html')
    expect(documentation.nav_config).to include('Compliance Foundations')
    expect(documentation.nav_config).to include('compliance/index.md')
    expect(documentation.nav_config).to include('compliance/learning/dcb0129-foundations.html')
  end
end
