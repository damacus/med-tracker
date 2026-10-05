# frozen_string_literal: true

require 'rails_helper'
require 'tmpdir'
require Rails.root.join('scripts/build_curated_product_yaml')

RSpec.describe CuratedProductYamlBuilder do
  def child_gummy_options
    {
      gtin: '5057753926137',
      display: 'Tesco Health 60 Childrens Multivitamins Strawberry Gummies',
      category: 'Vitamin',
      product_description: "Sugar-free children's multivitamin and mineral gummies.",
      warnings: 'Do not exceed the recommended daily dose.',
      dose_amount: '2',
      dose_unit: 'gummy',
      dose_frequency: 'Daily',
      dose_description: 'Children 3+ years',
      child_default: true,
      current_supply: '60',
      reorder_threshold: '14'
    }
  end

  describe '.build' do
    it 'includes product identifiers and classification' do
      product = described_class.build(child_gummy_options)

      expect(product).to include(
        'gtin' => '5057753926137',
        'display' => 'Tesco Health 60 Childrens Multivitamins Strawberry Gummies',
        'system' => 'Curated product catalog',
        'concept_class' => 'Food supplement',
        'category' => 'Vitamin'
      )
    end

    it 'includes the suggested dose with defaults and supplied values' do
      product = described_class.build(child_gummy_options)

      expect(product['suggested_doses']).to contain_exactly(
        a_hash_including(
          'amount' => 2,
          'unit' => 'gummy',
          'frequency' => 'Daily',
          'description' => 'Children 3+ years',
          'default_for_children' => true,
          'default_max_daily_doses' => 1,
          'default_min_hours_between_doses' => 24,
          'default_dose_cycle' => 'daily',
          'current_supply' => 60,
          'reorder_threshold' => 14
        )
      )
    end

    it 'requires enough product and dose data to make a usable curated entry' do
      expect { described_class.build(display: 'Missing GTIN') }.to raise_error(ArgumentError, /--gtin/)
    end
  end

  describe '.append!' do
    it 'appends a product once and protects against duplicate identifiers' do
      Dir.mktmpdir do |dir|
        path = File.join(dir, 'curated.yml')
        product = described_class.build(child_gummy_options)

        described_class.append!(path, product)

        expect(YAML.load_file(path)['products']).to contain_exactly(product)
        expect { described_class.append!(path, product) }.to raise_error(ArgumentError, /already exists/)
      end
    end
  end
end
