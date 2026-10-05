# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenProductsFacts::ResultBuilder do
  let(:barcode) { '5000436574637' }

  def product_payload(overrides = {})
    {
      'code' => barcode,
      'product_name' => 'Ibuprofen 200mg Tablets',
      'brands' => 'Tesco',
      'quantity' => '16 tablets',
      'categories_tags_en' => %w[Medicines Analgesics]
    }.merge(overrides)
  end

  def wrapped_payload(overrides = {})
    {
      'code' => barcode,
      'product' => product_payload.except('code').merge(overrides)
    }
  end

  describe '.catalog_entry_from_product' do
    it 'returns a catalog entry hash for a valid medicine product' do
      result = described_class.catalog_entry_from_product(barcode, product_payload)

      expect(result).to include(
        gtin: barcode,
        display: 'Ibuprofen 200mg Tablets (Tesco) 16 tablets',
        source: 'open_products_facts',
        concept_class: 'OTC Medicine'
      )
    end

    it 'returns nil when product name is blank' do
      result = described_class.catalog_entry_from_product(barcode, product_payload('product_name' => ''))
      expect(result).to be_nil
    end

    it 'returns nil for non-medicine products' do
      non_medicine = product_payload('categories_tags_en' => ['Laundry detergents'])
      expect(described_class.catalog_entry_from_product(barcode, non_medicine)).to be_nil
    end

    it 'unwraps nested product key' do
      result = described_class.catalog_entry_from_product(barcode, wrapped_payload)
      expect(result).to include(gtin: barcode, display: 'Ibuprofen 200mg Tablets (Tesco) 16 tablets')
    end

    it 'includes the system URL from Client::BASE_URL' do
      result = described_class.catalog_entry_from_product(barcode, product_payload)
      expect(result[:system]).to eq(OpenProductsFacts::Client::BASE_URL)
    end
  end

  describe '.normalized_payload' do
    it 'returns the nested product when product key is present' do
      outer = { 'code' => barcode, 'product' => { 'product_name' => 'Test', 'code' => nil } }
      payload = described_class.normalized_payload(outer)
      expect(payload['code']).to eq(barcode)
      expect(payload['product_name']).to eq('Test')
    end

    it 'returns the flat product when no product key is present' do
      flat = { 'code' => barcode, 'product_name' => 'Flat Product' }
      payload = described_class.normalized_payload(flat)
      expect(payload['code']).to eq(barcode)
      expect(payload['product_name']).to eq('Flat Product')
    end

    it 'prefers outer code over inner code' do
      outer = { 'code' => 'OUTER', 'product' => { 'product_name' => 'Test', 'code' => 'INNER' } }
      payload = described_class.normalized_payload(outer)
      expect(payload['code']).to eq('OUTER')
    end
  end

  describe '.medicine_product?' do
    it 'returns true for medicines category' do
      expect(described_class.medicine_product?('categories_tags_en' => ['medicines'])).to be true
    end

    it 'returns true for analgesics category' do
      expect(described_class.medicine_product?('categories_tags_en' => ['analgesics'])).to be true
    end

    it 'returns true for paracetamol keyword' do
      expect(described_class.medicine_product?('categories_tags_en' => ['paracetamol products'])).to be true
    end

    it 'returns true for ibuprofen keyword' do
      expect(described_class.medicine_product?('categories_tags_en' => ['ibuprofen'])).to be true
    end

    it 'returns true for pharmaceutical' do
      expect(described_class.medicine_product?('categories_tags_en' => ['pharmaceuticals'])).to be true
    end

    it 'returns false for unrelated categories' do
      expect(described_class.medicine_product?('categories_tags_en' => ['beverages'])).to be false
    end

    it 'returns false when categories_tags_en is absent' do
      expect(described_class.medicine_product?({})).to be false
    end

    it 'also checks categories_tags for medicine keywords' do
      payload = { 'categories_tags_en' => [], 'categories_tags' => ['medicines'] }
      expect(described_class.medicine_product?(payload)).to be true
    end

    it 'does not match category tokens that contain keywords as substrings only (word boundary enforcement)' do
      # "nonmedicines" or "paramedicine" should NOT match "medicine" as a whole word
      expect(described_class.medicine_product?('categories_tags_en' => ['nonmedicinal products'])).to be false
    end
  end

  describe '.display_for' do
    it 'joins name, brand, and quantity' do
      payload = product_payload
      expect(described_class.display_for(payload)).to eq('Ibuprofen 200mg Tablets (Tesco) 16 tablets')
    end

    it 'omits brand segment when brands is blank' do
      payload = product_payload('brands' => '')
      expect(described_class.display_for(payload)).to eq('Ibuprofen 200mg Tablets 16 tablets')
    end

    it 'omits quantity segment when quantity is blank' do
      payload = product_payload('quantity' => '')
      expect(described_class.display_for(payload)).to eq('Ibuprofen 200mg Tablets (Tesco)')
    end

    it 'returns just the name when brand and quantity are both blank' do
      payload = product_payload('brands' => '', 'quantity' => '')
      expect(described_class.display_for(payload)).to eq('Ibuprofen 200mg Tablets')
    end
  end

  describe '.product_name' do
    it 'returns stripped product name' do
      expect(described_class.product_name('product_name' => '  Ibuprofen  ')).to eq('Ibuprofen')
    end

    it 'returns empty string when product_name is absent' do
      expect(described_class.product_name({})).to eq('')
    end
  end

  describe '.brand_segment' do
    it 'wraps brand in parentheses' do
      expect(described_class.brand_segment('brands' => 'Tesco')).to eq('(Tesco)')
    end

    it 'returns nil when brands is blank' do
      expect(described_class.brand_segment('brands' => '')).to be_nil
    end

    it 'returns nil when brands is absent' do
      expect(described_class.brand_segment({})).to be_nil
    end
  end

  describe '.quantity_segment' do
    it 'returns quantity when present' do
      expect(described_class.quantity_segment('quantity' => '16 tablets')).to eq('16 tablets')
    end

    it 'returns nil when quantity is blank' do
      expect(described_class.quantity_segment('quantity' => '')).to be_nil
    end
  end

  describe '.category_tokens' do
    it 'combines categories_tags_en and categories_tags' do
      payload = { 'categories_tags_en' => %w[medicines analgesics], 'categories_tags' => ['drugs'] }
      expect(described_class.category_tokens(payload)).to contain_exactly('medicines', 'analgesics', 'drugs')
    end

    it 'returns empty array when both are absent' do
      expect(described_class.category_tokens({})).to eq([])
    end
  end
end
