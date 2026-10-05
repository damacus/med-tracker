# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFoodFacts::ResultBuilder do
  def product_payload(overrides = {})
    {
      'code' => '5021265221301',
      'product_name' => 'Wellman Original',
      'generic_name' => 'Daily multivitamin food supplement',
      'brands' => 'Vitabiotics',
      'quantity' => '30 tablets',
      'categories_tags_en' => %w[Supplements Vitamins]
    }.merge(overrides)
  end

  def wrapped_payload(overrides = {})
    {
      'code' => '5021265221301',
      'product' => product_payload.except('code').merge(overrides)
    }
  end

  describe '.search_result_from_product' do
    it 'returns a result hash for a valid supplement product' do
      result = described_class.search_result_from_product(product_payload)

      expect(result).to include(
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

    it 'returns nil when the product is nil' do
      expect(described_class.search_result_from_product(nil)).to be_nil
    end

    it 'returns nil when the product name is blank' do
      expect(described_class.search_result_from_product(product_payload('product_name' => ''))).to be_nil
    end

    it 'returns nil for non-supplement products' do
      non_supplement = product_payload('categories_tags_en' => ['Chocolate spreads'])
      expect(described_class.search_result_from_product(non_supplement)).to be_nil
    end

    it 'unwraps nested product key' do
      result = described_class.search_result_from_product(wrapped_payload)
      expect(result).to include(name: 'Wellman Original')
    end

    it 'sets code to nil (not the barcode field)' do
      result = described_class.search_result_from_product(product_payload)
      expect(result[:code]).to be_nil
    end
  end

  describe '.normalized_payload' do
    it 'returns the nested product when product key is present' do
      outer = { 'code' => 'X', 'product' => { 'product_name' => 'Test', 'code' => nil } }
      payload = described_class.normalized_payload(outer)
      expect(payload['code']).to eq('X')
      expect(payload['product_name']).to eq('Test')
    end

    it 'returns the flat product when no product key is present' do
      flat = { 'code' => 'Y', 'product_name' => 'Flat Product' }
      payload = described_class.normalized_payload(flat)
      expect(payload['code']).to eq('Y')
      expect(payload['product_name']).to eq('Flat Product')
    end

    it 'prefers outer code over inner product code' do
      outer = { 'code' => 'OUTER', 'product' => { 'product_name' => 'Test', 'code' => 'INNER' } }
      payload = described_class.normalized_payload(outer)
      expect(payload['code']).to eq('OUTER')
    end
  end

  describe '.supplement_product?' do
    it 'returns true for supplements category' do
      expect(described_class.supplement_product?('categories_tags_en' => ['supplements'])).to be true
    end

    it 'returns true for vitamins category' do
      expect(described_class.supplement_product?('categories_tags_en' => ['vitamins'])).to be true
    end

    it 'returns true for multivitamins' do
      expect(described_class.supplement_product?('categories_tags_en' => ['multivitamins'])).to be true
    end

    it 'returns false for unrelated categories' do
      expect(described_class.supplement_product?('categories_tags_en' => ['beverages'])).to be false
    end

    it 'returns false when categories_tags_en is absent' do
      expect(described_class.supplement_product?({})).to be false
    end

    it 'is case-insensitive' do
      expect(described_class.supplement_product?('categories_tags_en' => ['SUPPLEMENT'])).to be true
    end
  end

  describe '.display_for' do
    it 'joins name, brand, and quantity' do
      payload = product_payload
      expect(described_class.display_for(payload)).to eq('Wellman Original (Vitabiotics) 30 tablets')
    end

    it 'omits brand segment when brands is blank' do
      payload = product_payload('brands' => '')
      expect(described_class.display_for(payload)).to eq('Wellman Original 30 tablets')
    end

    it 'omits quantity segment when quantity is blank' do
      payload = product_payload('quantity' => '')
      expect(described_class.display_for(payload)).to eq('Wellman Original (Vitabiotics)')
    end

    it 'returns just the name when both brand and quantity are blank' do
      payload = product_payload('brands' => '', 'quantity' => '')
      expect(described_class.display_for(payload)).to eq('Wellman Original')
    end
  end

  describe '.package_quantity' do
    it 'extracts integer quantity' do
      expect(described_class.package_quantity('quantity' => '30 tablets')).to eq(30)
    end

    it 'extracts float quantity' do
      expect(described_class.package_quantity('quantity' => '1.5 litres')).to eq(1.5)
    end

    it 'handles comma as decimal separator' do
      expect(described_class.package_quantity('quantity' => '1,5 g')).to eq(1.5)
    end

    it 'returns nil when quantity is blank' do
      expect(described_class.package_quantity('quantity' => '')).to be_nil
    end
  end

  describe '.package_unit' do
    it 'normalizes tablet (singular)' do
      expect(described_class.package_unit('quantity' => '30 tablet')).to eq('tablet')
    end

    it 'normalizes tablets (plural)' do
      expect(described_class.package_unit('quantity' => '60 tablets')).to eq('tablet')
    end

    it 'normalizes capsules to capsule' do
      expect(described_class.package_unit('quantity' => '30 capsules')).to eq('capsule')
    end

    it 'normalizes millilitres to ml' do
      expect(described_class.package_unit('quantity' => '100 millilitres')).to eq('ml')
    end

    it 'normalizes grams to g' do
      expect(described_class.package_unit('quantity' => '200 grams')).to eq('g')
    end

    it 'returns nil for unknown units' do
      expect(described_class.package_unit('quantity' => '1 jar')).to be_nil
    end

    it 'returns nil when quantity is blank' do
      expect(described_class.package_unit('quantity' => '')).to be_nil
    end
  end

  describe '.generic_name' do
    it 'returns stripped generic name when present' do
      expect(described_class.generic_name('generic_name' => '  vitamin supplement  ')).to eq('vitamin supplement')
    end

    it 'returns nil when generic name is blank' do
      expect(described_class.generic_name('generic_name' => '')).to be_nil
    end

    it 'returns nil when generic name is absent' do
      expect(described_class.generic_name({})).to be_nil
    end
  end

  describe '.normalized_barcode' do
    it 'returns normalized GTIN for valid code' do
      result = described_class.normalized_barcode('code' => '5021265221301')
      expect(result).to eq('5021265221301')
    end

    it 'returns nil when code is blank' do
      expect(described_class.normalized_barcode('code' => '')).to be_nil
    end
  end

  describe '.normalize_quantity' do
    it 'returns integer for whole numbers' do
      expect(described_class.normalize_quantity('30')).to eq(30)
    end

    it 'returns float for decimal numbers' do
      expect(described_class.normalize_quantity('1.5')).to eq(1.5)
    end

    it 'converts comma decimal separator' do
      expect(described_class.normalize_quantity('2,5')).to eq(2.5)
    end

    it 'returns nil for blank input' do
      expect(described_class.normalize_quantity('')).to be_nil
      expect(described_class.normalize_quantity(nil)).to be_nil
    end
  end
end
