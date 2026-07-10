# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Nlm::RxClassClient do
  it 'keeps public ingredient, EPC, and chemical class data while excluding mechanism classes' do
    entries = client_with(rxclass_response).entries_for(['phenelzine'])

    expect(entries.sole).to include(
      'selection_term' => 'phenelzine',
      'rxcui' => '8123',
      'ingredient_name' => 'phenelzine',
      'classes' => contain_exactly(
        { 'id' => 'N1', 'name' => 'Monoamine Oxidase Inhibitor', 'type' => 'EPC' },
        { 'id' => 'N3', 'name' => 'Hydrazines', 'type' => 'CHEM' }
      )
    )
  end

  def client_with(response)
    client_class = Class.new(described_class) do
      define_method(:responses_for) { |_terms| [response] }
    end
    client_class.new
  end

  def rxclass_response
    {
      'rxclassDrugInfoList' => {
        'rxclassDrugInfo' => [
          drug_info('8123', 'phenelzine', 'N1', 'Monoamine Oxidase Inhibitor', 'EPC'),
          drug_info('8123', 'phenelzine', 'N2', 'Monoamine Oxidase Inhibitors', 'MOA'),
          drug_info('8123', 'phenelzine', 'N3', 'Hydrazines', 'CHEM')
        ]
      }
    }
  end

  def drug_info(rxcui, ingredient_name, class_id, class_name, class_type)
    {
      'minConcept' => { 'rxcui' => rxcui, 'name' => ingredient_name, 'tty' => 'IN' },
      'rxclassMinConceptItem' => { 'classId' => class_id, 'className' => class_name, 'classType' => class_type },
      'relaSource' => 'DAILYMED'
    }
  end
end
