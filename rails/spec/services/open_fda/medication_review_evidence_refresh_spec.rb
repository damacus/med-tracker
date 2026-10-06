# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::MedicationReviewEvidenceRefresh do
  subject(:result) { described_class.new(builder: builder, retrieved_on: retrieved_on).call }

  let(:retrieved_on) { Date.new(2026, 7, 10) }
  let(:builder) { instance_double(OpenFda::SnapshotBuilder) }

  before do
    create_evidence(label(set_id: 'refresh-unchanged', term: 'alpha'))
    create_evidence(label(set_id: 'refresh-changed', term: 'beta'))
    create_evidence(label(set_id: 'refresh-missing', term: 'gamma'))
    allow(builder).to receive(:call).and_return(live_snapshot)
  end

  def label(set_id:, term:, version: '1', evidence: 'No material source change.')
    {
      'set_id' => set_id,
      'id' => "#{set_id}-version",
      'selection_term' => term,
      'effective_time' => '20260701',
      'version' => version,
      'drug_interactions' => [evidence],
      'openfda' => {
        'brand_name' => [term.titleize],
        'generic_name' => [term],
        'substance_name' => [term.upcase]
      }
    }
  end

  def create_evidence(label)
    MedicationReviewEvidenceRecord.create!(OpenFda::EvidenceAttributes.new(retrieved_on: retrieved_on - 7).call(label))
  end

  it 'reports new, changed, unchanged, and missing records' do
    expect(result).to include(
      source_last_updated: retrieved_on, label_count: 3, created_count: 1,
      updated_count: 1, unchanged_count: 1, missing_count: 1
    )
    expect(result.fetch(:changes)).to include(*expected_changes)
  end

  it 'imports the changed public label evidence' do
    result

    expect(MedicationReviewEvidenceRecord.find_by!(source_record_id: 'refresh-changed')).to have_attributes(
      source_version: '2', evidence_text: 'Monitor beta with delta.', retrieved_on: retrieved_on
    )
  end

  def live_snapshot
    {
      'openfda_last_updated' => '2026-07-10', 'generated_on' => retrieved_on.iso8601,
      'labels' => [
        label(set_id: 'refresh-unchanged', term: 'alpha'),
        label(set_id: 'refresh-changed', term: 'beta', version: '2', evidence: 'Monitor beta with delta.'),
        label(set_id: 'refresh-new', term: 'delta')
      ]
    }
  end

  def expected_changes
    [
      include(type: 'updated', source_record_id: 'refresh-changed', from_version: '1', to_version: '2'),
      include(type: 'created', source_record_id: 'refresh-new'),
      include(type: 'missing', source_record_id: 'refresh-missing')
    ]
  end
end
