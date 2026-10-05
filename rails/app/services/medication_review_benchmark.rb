# frozen_string_literal: true

class MedicationReviewBenchmark
  def initialize(records:, selection:, cases:)
    @corpus = MedicationReviewEvidenceCorpus.new(records)
    @selection = selection
    @cases = cases
  end

  def call
    {
      'inventory' => inventory_report,
      'benchmark' => benchmark_report
    }
  end

  private

  attr_reader :corpus, :selection, :cases

  def inventory_report
    results = selected_pair_results
    matches = results.flat_map { |result| result.fetch(:matches) }
    {
      'candidate_pair_count' => results.size,
      'matched_pair_count' => results.count { |result| result.fetch(:matches).any? },
      'match_count' => matches.size,
      'match_types' => tally(matches, &:match_type),
      'source_instructions' => tally(matches, &:source_instruction),
      'risk_levels' => tally(matches, &:risk_level),
      'match_confidences' => tally(matches, &:match_confidence),
      'matched_pairs' => matched_pair_details(results)
    }
  end

  def selected_pair_results
    selection.combination(2).map do |first, second|
      { first: first, second: second, matches: corpus.matches_for(first, second) }
    end
  end

  def benchmark_report
    results = cases.map { |benchmark_case| evaluate_case(benchmark_case) }
    counts = outcome_counts(results)
    counts.merge(
      'case_count' => results.size,
      'precision' => ratio(counts.fetch('true_positives'), counts.fetch('false_positives')),
      'recall' => ratio(counts.fetch('true_positives'), counts.fetch('false_negatives')),
      'cases' => results
    )
  end

  def evaluate_case(benchmark_case)
    matches = corpus.matches_for(benchmark_case.fetch('first'), benchmark_case.fetch('second'))
    actual_match = matches.any?
    benchmark_case.merge(
      'actual_match' => actual_match,
      'outcome' => outcome(benchmark_case.fetch('expected_match'), actual_match),
      'matches' => matches.map { |match| match_details(match) }
    )
  end

  def outcome(expected_match, actual_match)
    return 'true_positive' if expected_match && actual_match
    return 'false_negative' if expected_match
    return 'false_positive' if actual_match

    'true_negative'
  end

  def outcome_counts(results)
    counts = results.each_with_object(Hash.new(0)) { |result, total| total[result.fetch('outcome')] += 1 }
    {
      'true_positives' => counts['true_positive'],
      'true_negatives' => counts['true_negative'],
      'false_positives' => counts['false_positive'],
      'false_negatives' => counts['false_negative']
    }
  end

  def ratio(true_positives, errors)
    denominator = true_positives + errors
    denominator.zero? ? nil : (true_positives.to_f / denominator).round(4)
  end

  def match_details(match)
    {
      'source_record_id' => match.evidence.source_record_id,
      'matched_term' => match.matched_term,
      'match_type' => match.match_type,
      'match_confidence' => match.match_confidence,
      'source_instruction' => match.source_instruction,
      'risk_level' => match.risk_level
    }
  end

  def matched_pair_details(results)
    results.filter_map do |result|
      next if result.fetch(:matches).empty?

      {
        'first' => result.fetch(:first),
        'second' => result.fetch(:second),
        'matches' => result.fetch(:matches).map { |match| match_details(match) }
      }
    end
  end

  def tally(values)
    values.each_with_object(Hash.new(0)) { |value, counts| counts[yield(value)] += 1 }.sort.to_h
  end
end
