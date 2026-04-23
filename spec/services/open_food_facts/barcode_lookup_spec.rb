# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFoodFacts::BarcodeLookup do
  subject(:lookup) { described_class.new(client: client) }

  let(:client) { instance_double(OpenFoodFacts::Client) }

  def expect_wellman_result(result)
    expect(result).to include(
      name: 'Wellman Original',
      display: 'Wellman Original (Vitabiotics) 30 tablets',
      category: 'Supplement',
      package_size: '30 tablets',
      package_quantity: 30,
      package_unit: 'tablet',
      concept_class: 'Supplement',
      source: 'open_food_facts'
    )
  end

  it 'builds a supplement result from an Open Food Facts product payload' do
    allow(client).to receive(:product).with('5021265221301').and_return(
      {
        'product' => {
          'product_name' => 'Wellman Original',
          'brands' => 'Vitabiotics',
          'quantity' => '30 tablets',
          'categories_tags_en' => %w[Supplements Vitamins]
        }
      }
    )

    expect_wellman_result(lookup.lookup('5021265221301'))
  end

  it 'returns nil when Open Food Facts does not know the barcode' do
    allow(client).to receive(:product).with('5021265221301').and_return(nil)

    expect(lookup.lookup('5021265221301')).to be_nil
  end

  it 'returns nil for non-supplement Open Food Facts products' do
    allow(client).to receive(:product).with('3017620422003').and_return(
      {
        'product' => {
          'product_name' => 'Nutella',
          'brands' => 'Ferrero',
          'quantity' => '400 g',
          'categories_tags_en' => ['Spreads', 'Chocolate spreads']
        }
      }
    )

    expect(lookup.lookup('3017620422003')).to be_nil
  end
end
