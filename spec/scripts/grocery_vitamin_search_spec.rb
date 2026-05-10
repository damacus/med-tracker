# frozen_string_literal: true

require 'spec_helper'
require Rails.root.join('scripts/search_grocery_vitamins')

RSpec.describe GroceryVitaminSearch do
  describe GroceryVitaminSearch::ResultFilter do
    subject(:filter) { described_class.new }

    it 'identifies vitamin products by category tag' do
      product = { 'categories_tags' => ['en:vitamins', 'en:food-supplements'] }
      expect(filter.vitamin?(product)).to be true
    end

    it 'identifies mineral supplement products' do
      product = { 'categories_tags' => ['en:mineral-supplements'] }
      expect(filter.vitamin?(product)).to be true
    end

    it 'excludes products without vitamin or supplement categories' do
      product = { 'categories_tags' => ['en:beverages', 'en:soft-drinks'] }
      expect(filter.vitamin?(product)).to be false
    end

    it 'handles products with no category tags' do
      product = { 'categories_tags' => [] }
      expect(filter.vitamin?(product)).to be false
    end
  end

  describe GroceryVitaminSearch::Formatter do
    subject(:formatter) { described_class.new }

    let(:product) do
      {
        'code' => '5057753926137',
        'brands' => ['Tesco'],
        'product_name' => 'Childrens Multivitamins Strawberry Gummies',
        'quantity' => '60 gummies',
        'categories_tags' => ['en:vitamins']
      }
    end

    describe '#format_table' do
      it 'returns a no-results message for empty input' do
        expect(formatter.format_table([])).to eq("No results found.\n")
      end

      it 'includes product code, brand and name in the table' do
        output = formatter.format_table([product])
        expect(output).to include('5057753926137')
        expect(output).to include('Tesco')
        expect(output).to include('Childrens Multivitamins')
      end
    end

    describe '#format_yaml_hints' do
      it 'returns a no-results comment for empty input' do
        expect(formatter.format_yaml_hints([])).to eq("# No results found.\n")
      end

      it 'includes the GTIN and display name in the build command hint' do
        output = formatter.format_yaml_hints([product])
        expect(output).to include('5057753926137')
        expect(output).to include('build_curated_product_yaml.rb')
        expect(output).to include('--gtin')
      end
    end
  end
end
