# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewTerminology do
  let(:nitrate_entry) do
    {
      'selection_term' => 'nitroglycerin', 'rxcui' => '4917', 'ingredient_name' => 'nitroglycerin',
      'classes' => [
        { 'id' => 'D009566', 'name' => 'Nitrates', 'type' => 'CHEM' },
        { 'id' => 'N0000175415', 'name' => 'Nitrate Vasodilator', 'type' => 'EPC' }
      ]
    }
  end
  let(:maoi_entry) do
    {
      'selection_term' => 'phenelzine', 'rxcui' => '8123', 'ingredient_name' => 'phenelzine',
      'classes' => [{ 'id' => 'N1', 'name' => 'Monoamine Oxidase Inhibitor', 'type' => 'EPC' }]
    }
  end
  let(:maoi_alias) do
    {
      'canonical_class' => 'monoamine oxidase inhibitor', 'terms' => ['mao inhibitor', 'mao a inhibitor'],
      'sources' => [{ 'set_id' => 'public-label', 'version' => '1' }]
    }
  end

  it 'returns ingredient aliases and public non-mechanism classes independently of label records' do
    terminology = described_class.new(entries: [nitrate_entry], aliases: [])
    identity = terminology.identity_for('Nitroglycerin 400 micrograms tablets')

    expect(identity).to eq(terms: ['nitroglycerin'], classes: ['nitrate', 'nitrate vasodilator'])
  end

  it 'adds source-backed class aliases without creating interaction rules' do
    terminology = described_class.new(entries: [maoi_entry], aliases: [maoi_alias])

    expect(terminology.identity_for('Phenelzine')).to include(
      classes: ['monoamine oxidase inhibitor', 'mao inhibitor', 'mao a inhibitor']
    )
  end
end
