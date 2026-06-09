# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::ResultBuilder do
  subject(:builder) { described_class.new(query: '  Para ') }

  def score_for(title, secondary: [])
    builder.build(type: :medication, title: title, subtitle: 's', path: '/p', secondary_values: secondary).score
  end

  it 'strips leading/trailing whitespace from the query before matching' do
    # builder query is '  Para ' — after strip+downcase it becomes 'para'
    # If strip were omitted the query would be '  para ' and nothing would match
    exact_builder = described_class.new(query: '  Para ')
    result = exact_builder.build(type: :medication, title: 'para', subtitle: 's', path: '/p')
    expect(result.score).to eq(100)
  end

  it 'scores an exact (case-insensitive) title 100' do
    expect(score_for('para')).to eq(100)
    expect(score_for('PARA')).to eq(100)
  end

  it 'scores an exact match with surrounding spaces in the title 100' do
    expect(score_for('  para  ')).to eq(100)
  end

  it 'normalises secondary values to lowercase for matching' do
    # kills the downcase -> upcase mutant in normalize
    expect(score_for('Ibuprofen', secondary: ['PARA brand'])).to eq(40)
  end

  # Verifies both that prefix → 80 AND that prefix beats substring (not 60)
  it 'scores a prefix match 80 (higher than substring 60)' do
    expect(score_for('Paracetamol')).to eq(80)
  end

  it 'scores a substring (non-prefix) match 60' do
    expect(score_for('Co-paracetamol')).to eq(60)
  end

  it 'scores a secondary-value match 40 when the title does not match' do
    expect(score_for('Ibuprofen', secondary: ['Para brand'])).to eq(40)
  end

  # Verifies both that 0 is returned AND that a non-matching secondary doesn't score 40
  it 'scores 0 when nothing matches (title or secondary)' do
    expect(score_for('Ibuprofen', secondary: ['Nurofen'])).to eq(0)
  end

  it 'prefers exact over prefix (exact title scores 100 not 80)' do
    # 'para' exactly matches normalised query 'para', should be 100 not 80
    expect(score_for('para')).to eq(100)
  end

  it 'prefers substring over secondary-value match (60 vs 40)' do
    # The title contains the query, so secondary values should not drag the score to 40
    expect(score_for('Co-paracetamol', secondary: ['Para brand'])).to eq(60)
  end

  it 'builds a Result carrying the supplied fields' do
    result = builder.build(type: :person, title: 'Para', subtitle: 'sub', path: '/people/1')
    expect(result).to have_attributes(type: :person, title: 'Para', subtitle: 'sub', path: '/people/1', score: 100)
  end

  it 'scores 40 when only one of multiple secondary values matches (not requiring all to match)' do
    # kills the any? -> all? mutant: 'Nurofen' doesn't contain 'para', 'Para brand' does
    expect(score_for('Ibuprofen', secondary: ['Nurofen', 'Para brand'])).to eq(40)
  end

  describe '#rescore' do
    it 'recomputes score from secondary values while preserving all identity fields' do
      original = builder.build(type: :medication, title: 'Ibuprofen', subtitle: 'brand sub', path: '/m/1')
      rescored = builder.rescore(original, secondary_values: ['Para'])
      expect(rescored).to have_attributes(
        type: :medication, title: 'Ibuprofen', subtitle: 'brand sub', path: '/m/1', score: 40
      )
    end

    it 'recomputes to 0 when secondary values do not match either' do
      original = builder.build(type: :medication, title: 'Ibuprofen', subtitle: 'sub', path: '/m/1')
      rescored = builder.rescore(original, secondary_values: ['Nurofen'])
      expect(rescored.score).to eq(0)
    end

    it 'recomputes to 100 when the original title is an exact match during rescore' do
      original = builder.build(type: :medication, title: 'para', subtitle: 'sub', path: '/m/1')
      rescored = builder.rescore(original)
      expect(rescored.score).to eq(100)
    end

    it 'defaults secondary_values to empty array (no score from secondary)' do
      original = builder.build(type: :medication, title: 'Ibuprofen', subtitle: 'sub', path: '/m/1')
      # No secondary_values passed - should be 0, not crash
      expect(builder.rescore(original).score).to eq(0)
    end

    it 'accepts an explicit empty secondary_values and scores by title only' do
      original = builder.build(type: :medication, title: 'Ibuprofen', subtitle: 'sub', path: '/m/1')
      rescored = builder.rescore(original, secondary_values: [])
      expect(rescored.score).to eq(0)
    end
  end

  describe GlobalSearch::Result do
    it 'serialises to the public JSON shape' do
      json = described_class.new(type: :medication, title: 'Para', subtitle: 's', path: '/p', score: 100).as_json
      expect(json).to eq(type: :medication, title: 'Para', subtitle: 's', path: '/p', score: 100)
    end

    it 'exposes all defined attributes' do
      result = described_class.new(type: :medication, title: 'T', subtitle: 'S', path: '/x', score: 60)
      expect(result).to have_attributes(type: :medication, title: 'T', subtitle: 'S', path: '/x', score: 60)
    end
  end
end
