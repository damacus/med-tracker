# frozen_string_literal: true

require 'rails_helper'

RSpec.describe BarcodeCatalog::CuratedProducts do
  # Reload the YAML before each example so memoized state doesn't leak
  before { described_class.instance_variable_set(:@products, nil) }

  describe '.lookup_gtin' do
    it 'finds Pregnacare Plus by GTIN' do
      result = described_class.lookup_gtin('5021265232062')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Pregnacare Plus tablets and capsules (Vitabiotics Ltd)')
      expect(result.code).to eq('35394411000001103')
    end

    it 'finds Calpol Vapour Plug by GTIN' do
      result = described_class.lookup_gtin('3574661646435')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Calpol Vapour Plug & Nightlight + 3 Refill Pads')
    end

    it 'finds Tesco Childrens Multivitamin Gummies by GTIN' do
      result = described_class.lookup_gtin('5057753926137')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Tesco Health 60 Childrens Multivitamins Strawberry Gummies')
    end

    it 'finds Tesco Paracetamol 500mg Tablets by GTIN' do
      result = described_class.lookup_gtin('5031021971579')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Tesco Health Paracetamol 500mg Tablets')
      expect(result.concept_class).to eq('Analgesic')
    end

    it 'finds Tesco Ibuprofen 200mg Tablets by GTIN' do
      result = described_class.lookup_gtin('5000436574637')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Tesco Health Ibuprofen 200mg Pain Relief Tablets 16s')
      expect(result.concept_class).to eq('Analgesic')
    end

    it 'finds Tesco Ibuprofen 200mg Liquid Capsules by GTIN' do
      result = described_class.lookup_gtin('5000358536010')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Tesco Health Ibuprofen 200mg Liquid Capsules 16s')
    end

    it 'finds Tesco Rapid Pain Relief Ibuprofen Lysine Tablets by GTIN' do
      result = described_class.lookup_gtin('5052003056763')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Tesco Health Rapid Pain Relief Tablets - Ibuprofen Lysine 12s')
    end

    it 'finds Tesco Migraine Relief Ibuprofen Lysine Tablets by GTIN' do
      result = described_class.lookup_gtin('5051277362020')
      expect(result).not_to be_nil
      expect(result.display_name).to eq('Tesco Health Migraine Relief Tablets - Ibuprofen Lysine 12s')
    end

    it 'returns nil for an unknown GTIN' do
      expect(described_class.lookup_gtin('0000000000000')).to be_nil
    end
  end

  describe '.find' do
    it 'finds Calpol Six Plus by dm+d code' do
      result = described_class.find(code: '316811000001106')
      expect(result).not_to be_nil
      expect(result.display_name).to match(/Calpol Six Plus/)
    end
  end

  describe 'YAML integrity' do
    subject(:products) { described_class.send(:products) }

    it 'loads without error' do
      expect { products }.not_to raise_error
    end

    it 'every product has a display name' do
      expect(products.map(&:display_name)).to all(be_present)
    end

    it 'every product has at least one suggested dose' do
      products.each do |product|
        expect(product.suggested_doses).not_to be_empty,
                                               "#{product.display_name} has no suggested doses"
      end
    end

    it 'every GTIN-based product has a valid 13–14 digit GTIN' do
      gtin_products = products.select(&:gtin)
      gtin_products.each do |product|
        expect(product.gtin).to match(/\A\d{13,14}\z/),
                                "#{product.display_name} has invalid GTIN: #{product.gtin}"
      end
    end

    it 'every ibuprofen product enforces a 4-hour minimum between doses' do
      ibuprofen_products = products.select { |p| p.display_name.match?(/ibuprofen/i) }
      expect(ibuprofen_products).not_to be_empty

      ibuprofen_products.each do |product|
        product.suggested_doses.each do |dose|
          msg = "#{product.display_name}: OTC ibuprofen requires <= 4 hours between doses"
          expect(dose.default_min_hours_between_doses).to be <= 4, msg
        end
      end
    end

    it 'every paracetamol product enforces a 4-hour minimum between doses' do
      paracetamol_products = products.select { |p| p.display_name.match?(/paracetamol/i) }
      expect(paracetamol_products).not_to be_empty

      paracetamol_products.each do |product|
        product.suggested_doses.each do |dose|
          msg = "#{product.display_name}: paracetamol requires <= 4 hours between doses"
          expect(dose.default_min_hours_between_doses).to be <= 4, msg
        end
      end
    end
  end
end
