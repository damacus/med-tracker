# frozen_string_literal: true

namespace :medication_review_evidence do
  desc 'Regenerate the committed public drug-label snapshot from its versioned selection manifest'
  task snapshot: :environment do
    path = OpenFda::SnapshotClient::DEFAULT_PATH
    snapshot = OpenFda::SnapshotBuilder.new.call
    FileUtils.mkdir_p(path.dirname)
    File.write(path, "#{JSON.pretty_generate(snapshot)}\n")
    puts "Wrote #{snapshot.fetch('labels').size} public labels to #{path}."
  end

  desc 'Regenerate the committed no-license NLM RxClass terminology snapshot'
  task terminology_snapshot: :environment do
    path = MedicationReviewTerminology::DEFAULT_PATH
    snapshot = Nlm::RxClassSnapshotBuilder.new.call
    File.write(path, "#{JSON.pretty_generate(snapshot)}\n")
    puts "Wrote #{snapshot.fetch('entries').size} public terminology entries to #{path}."
  end

  desc 'Import public drug-interaction label sections into the local evidence store'
  task import: :environment do
    limit = Integer(ENV['LIMIT']) if ENV['LIMIT'].present?
    records = OpenFda::MedicationReviewEvidenceImporter.new.call(limit: limit)
    puts "Imported #{records.size} public label evidence records."
  end

  desc 'Measure automatic detection over the committed snapshot and finite benchmark cases'
  task benchmark: :environment do
    path = Rails.root.join('data/medication_reviews/benchmark_report.json')
    report = MedicationReviewBenchmarkRunner.new.call
    File.write(path, "#{JSON.pretty_generate(report)}\n")
    puts "Measured #{report.dig('inventory', 'candidate_pair_count')} pairs and " \
         "#{report.dig('benchmark', 'case_count')} benchmark cases."
  end

  desc 'Refresh public label evidence and persist a source change report'
  task refresh: :environment do
    run = MedicationReviewEvidenceRefreshJob.perform_now
    puts "Refreshed #{run.label_count} labels: #{run.created_count} new, #{run.updated_count} changed, " \
         "#{run.unchanged_count} unchanged, #{run.missing_count} missing."
  end
end
