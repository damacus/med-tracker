# frozen_string_literal: true

class MedicationReviewBenchmarkRunner
  DEFAULT_SNAPSHOT_PATH = OpenFda::SnapshotClient::DEFAULT_PATH
  DEFAULT_CASES_PATH = Rails.root.join('data/medication_reviews/benchmark_cases.yml')

  def initialize(snapshot_path: DEFAULT_SNAPSHOT_PATH, cases_path: DEFAULT_CASES_PATH,
                 manifest: OpenFda::SnapshotManifest.new)
    @snapshot_path = snapshot_path
    @cases_path = cases_path
    @manifest = manifest
  end

  def call
    snapshot = JSON.parse(File.read(snapshot_path))
    records = evidence_records(snapshot)
    report = MedicationReviewBenchmark.new(
      records: records,
      selection: manifest.selection,
      cases: benchmark_cases
    ).call
    report.merge(
      'snapshot' => {
        'selection_version' => snapshot.fetch('selection_version'),
        'generated_on' => snapshot.fetch('generated_on'),
        'openfda_last_updated' => snapshot.fetch('openfda_last_updated'),
        'label_count' => records.size
      }
    )
  end

  private

  attr_reader :snapshot_path, :cases_path, :manifest

  def evidence_records(snapshot)
    mapper = OpenFda::EvidenceAttributes.new(retrieved_on: Date.iso8601(snapshot.fetch('generated_on')))
    snapshot.fetch('labels').map { |label| MedicationReviewEvidenceRecord.new(mapper.call(label)) }
  end

  def benchmark_cases
    YAML.safe_load_file(cases_path).fetch('cases')
  end
end
