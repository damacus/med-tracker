# frozen_string_literal: true

class MedicationReviewTerminology
  DEFAULT_PATH = Rails.root.join('data/medication_reviews/rxclass_terminology.json')
  DEFAULT_ALIASES_PATH = Rails.root.join('config/medication_review_terminology_aliases.yml')

  def initialize(entries: nil, aliases: nil, path: DEFAULT_PATH, aliases_path: DEFAULT_ALIASES_PATH)
    @entries = entries || JSON.parse(File.read(path)).fetch('entries')
    @aliases = aliases || YAML.safe_load_file(aliases_path).fetch('aliases')
  end

  def identity_for(medication_name)
    medication_term = MedicationReviewTermNormalizer.medication(medication_name)
    matching_entries = entries.select { |entry| entry_matches?(entry, medication_term) }
    {
      terms: matching_entries.flat_map { |entry| entry_terms(entry) }.compact_blank.uniq,
      classes: matching_entries.flat_map { |entry| class_names(entry) }.compact_blank.uniq
    }
  end

  private

  attr_reader :entries, :aliases

  def entry_matches?(entry, medication_term)
    entry_terms(entry).any? { |term| overlapping_terms?(medication_term, term) }
  end

  def entry_terms(entry)
    [entry['selection_term'], entry['ingredient_name']].map { |term| MedicationReviewTermNormalizer.label(term) }
  end

  def class_names(entry)
    canonical_classes = entry.fetch('classes').map do |item|
      MedicationReviewTermNormalizer.label(item.fetch('name')).singularize
    end
    (canonical_classes + aliases_for(canonical_classes)).uniq
  end

  def aliases_for(canonical_classes)
    matching_aliases = aliases.filter_map do |group|
      canonical = MedicationReviewTermNormalizer.label(group.fetch('canonical_class')).singularize
      group.fetch('terms') if canonical_classes.include?(canonical)
    end
    matching_aliases.flatten.map { |term| MedicationReviewTermNormalizer.label(term) }
  end

  def overlapping_terms?(first, second)
    padded_first = " #{first} "
    padded_second = " #{second} "
    padded_first.include?(padded_second) || padded_second.include?(padded_first)
  end
end
