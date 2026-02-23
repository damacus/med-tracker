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
    it { is_expected.to allow_value('sachet').for(:dosage_unit) }
    it { is_expected.not_to allow_value('capsule').for(:dosage_unit) }

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
        current_supply: current_supply,
        stock: 200,
        reorder_threshold: 50
      )
    end

    context 'when current_supply is below the reorder threshold' do
      let(:current_supply) { 25 }

      it 'returns true' do
        expect(medicine.low_stock?).to be(true)
      end
    end

    context 'when current_supply meets the reorder threshold' do
      let(:current_supply) { 50 }

      it 'returns true' do
        expect(medicine.low_stock?).to be(true)
      end
    end

    context 'when current_supply is above the reorder threshold' do
      let(:current_supply) { 75 }

      it 'returns false' do
        expect(medicine.low_stock?).to be(false)
      end
    end
  end

  describe '#out_of_stock?' do
    subject(:medicine) { described_class.new(current_supply: current_supply) }

    context 'when current_supply is 0' do
      let(:current_supply) { 0 }

      it { is_expected.to be_out_of_stock }
    end

    context 'when current_supply is positive' do
      let(:current_supply) { 1 }

      it { is_expected.not_to be_out_of_stock }
    end

    context 'when current_supply is nil (untracked)' do
      let(:current_supply) { nil }

      it { is_expected.not_to be_out_of_stock }
    end
  end
end
