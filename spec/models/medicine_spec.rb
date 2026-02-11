# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Medicine do
  subject(:medicine) do
    described_class.new(
      name: 'Ibuprofen',
      current_supply: 200,
      stock: 200,
      reorder_threshold: 50
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:name) }
    it { is_expected.not_to validate_presence_of(:current_supply) }

    it do
      expect(medicine).to validate_numericality_of(:current_supply)
        .only_integer
        .is_greater_than_or_equal_to(0)
        .allow_nil
    end

    it { is_expected.to validate_numericality_of(:stock).only_integer.is_greater_than_or_equal_to(0).allow_nil }
    it { is_expected.to validate_numericality_of(:reorder_threshold).only_integer.is_greater_than_or_equal_to(0) }
  end

  describe 'associations' do
    it { is_expected.to have_many(:dosages).dependent(:destroy) }
    it { is_expected.to have_many(:prescriptions).dependent(:destroy) }
  end

  describe '#low_stock?' do
    subject(:medicine) do
      described_class.new(
        name: 'Ibuprofen',
        current_supply: 200,
        stock: stock,
        reorder_threshold: 50
      )
    end

    context 'when stock is below the reorder threshold' do
      let(:stock) { 25 }

      it 'returns true' do
        expect(medicine.low_stock?).to be(true)
      end
    end

    context 'when stock meets the reorder threshold' do
      let(:stock) { 50 }

      it 'returns true' do
        expect(medicine.low_stock?).to be(true)
      end
    end

    context 'when stock is above the reorder threshold' do
      let(:stock) { 75 }

      it 'returns false' do
        expect(medicine.low_stock?).to be(false)
      end
    end
  end

  describe '#out_of_stock?' do
    subject(:medicine) { described_class.new(stock: stock) }

    context 'when stock is 0' do
      let(:stock) { 0 }

      it { is_expected.to be_out_of_stock }
    end

    context 'when stock is positive' do
      let(:stock) { 1 }

      it { is_expected.not_to be_out_of_stock }
    end

    context 'when stock is nil (untracked)' do
      let(:stock) { nil }

      it { is_expected.not_to be_out_of_stock }
    end
  end
end
