# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFoodFacts::Search do
  subject(:search) { described_class.new(client: client) }

  let(:client) { instance_double(OpenFoodFacts::Client) }

  def wellman_result_matcher
    a_hash_including(
      name: 'Wellman Original',
      description: 'Daily multivitamin food supplement',
      display: 'Wellman Original (Vitabiotics) 30 tablets',
      barcode: '5021265221301',
      category: 'Supplement',
      package_size: '30 tablets',
      package_quantity: 30,
      package_unit: 'tablet',
      concept_class: 'Supplement',
      source: 'open_food_facts'
    )
  end

  def product_payload(attributes)
    {
      'code' => attributes.fetch(:code),
      'product_name' => attributes.fetch(:name),
      'generic_name' => attributes.fetch(:generic_name),
      'brands' => attributes.fetch(:brands),
      'quantity' => attributes.fetch(:quantity),
      'categories_tags_en' => attributes.fetch(:categories)
    }
  end

  it 'builds supplement search results from Open Food Facts product matches' do
    allow(client).to receive(:search_products).with('wellman', page_size: 10).and_return(
      [product_payload(code: '5021265221301', name: 'Wellman Original', brands: 'Vitabiotics',
                       generic_name: 'Daily multivitamin food supplement', quantity: '30 tablets',
                       categories: %w[Supplements Vitamins])]
    )

    result = search.search('wellman')

    expect(result).to contain_exactly(wellman_result_matcher)
  end

  it 'filters out non-supplement search results' do
    allow(client).to receive(:search_products).with('nutella', page_size: 10).and_return(
      [product_payload(code: '3017620422003', name: 'Nutella', brands: 'Ferrero', generic_name: 'Hazelnut cocoa spread',
                       quantity: '400 g', categories: ['Chocolate spreads'])]
    )

    expect(search.search('nutella')).to eq([])
  end
end
