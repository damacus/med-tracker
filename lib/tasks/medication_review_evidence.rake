# frozen_string_literal: true

namespace :medication_review_evidence do
  desc 'Import public drug-interaction label sections into the local evidence store'
  task import: :environment do
    limit = Integer(ENV.fetch('LIMIT', 80))
    records = OpenFda::MedicationReviewEvidenceImporter.new.call(limit: limit)
    puts "Imported #{records.size} public label evidence records."
  end
end
