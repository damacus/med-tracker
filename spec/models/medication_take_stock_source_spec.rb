# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTakeStockSource do
  let(:take) do
    instance_double(
      MedicationTake,
      dose_amount: 1,
      dose_unit: 'tablet',
      source: nil,
      taken_at: Time.zone.now
    )
  end
  let(:inventory) { instance_double(Medication, present?: true, blank?: false) }
  let(:resolver) { instance_double(InventoryDosageOptionResolver, tracked_inventory?: false, call: nil) }

  before do
    allow(InventoryDosageOptionResolver).to receive(:new).and_return(resolver)
  end

  subject(:stock_source) { described_class.new(take: take, inventory: inventory) }

  describe '#tracked?' do
    context 'when inventory is blank' do
      let(:inventory) { nil }

      it { is_expected.not_to be_tracked }
    end

    context 'when inventory is present but has no current_supply and no dosage_option' do
      before do
        allow(inventory).to receive(:current_supply).and_return(nil)
        allow(resolver).to receive(:call).and_return(nil)
      end

      it { is_expected.not_to be_tracked }
    end

    context 'when inventory has a current_supply' do
      before do
        allow(inventory).to receive(:current_supply).and_return(10)
      end

      it { is_expected.to be_tracked }
    end

    context 'when inventory has a dosage_option resolved' do
      let(:dosage_option) { instance_double(Dosage, present?: true) }

      before do
        allow(inventory).to receive(:current_supply).and_return(nil)
        allow(resolver).to receive(:call).and_return(dosage_option)
      end

      it { is_expected.to be_tracked }
    end
  end

  describe '#selected_dose?' do
    context 'when inventory is blank' do
      let(:inventory) { nil }

      it { is_expected.to be_selected_dose }
    end

    context 'when inventory is tracked by dosage records but resolver returns nothing' do
      before do
        allow(inventory).to receive(:current_supply).and_return(nil)
        allow(resolver).to receive(:tracked_inventory?).and_return(true)
        allow(resolver).to receive(:call).and_return(nil)
      end

      it { is_expected.not_to be_selected_dose }
    end

    context 'when not tracking by dosage records' do
      before do
        allow(resolver).to receive(:tracked_inventory?).and_return(false)
      end

      it { is_expected.to be_selected_dose }
    end

    context 'when dosage_option is resolved' do
      let(:dosage_option) { instance_double(Dosage) }

      before do
        allow(resolver).to receive(:tracked_inventory?).and_return(true)
        allow(resolver).to receive(:call).and_return(dosage_option)
      end

      it { is_expected.to be_selected_dose }
    end
  end

  describe '#in_stock?' do
    context 'when inventory is blank' do
      let(:inventory) { nil }

      it { is_expected.to be_in_stock }
    end

    context 'when tracked dosage inventory but dosage_option missing (missing tracked dose)' do
      before do
        allow(inventory).to receive(:current_supply).and_return(nil)
        allow(resolver).to receive(:tracked_inventory?).and_return(true)
        allow(resolver).to receive(:call).and_return(nil)
      end

      it { is_expected.to be_in_stock }
    end

    context 'when dosage_option has current_supply and is sufficient' do
      let(:dosage_option) { instance_double(Dosage, current_supply: 5, unit: 'tablet') }

      before do
        allow(resolver).to receive(:tracked_inventory?).and_return(true)
        allow(resolver).to receive(:call).and_return(dosage_option)
        allow(MedicationStockConsumption).to receive(:sufficient?).and_return(true)
      end

      it { is_expected.to be_in_stock }
    end

    context 'when dosage_option has current_supply but insufficient' do
      let(:dosage_option) { instance_double(Dosage, current_supply: 5, unit: 'tablet') }

      before do
        allow(resolver).to receive(:tracked_inventory?).and_return(true)
        allow(resolver).to receive(:call).and_return(dosage_option)
        allow(MedicationStockConsumption).to receive(:sufficient?).and_return(false)
      end

      it { is_expected.not_to be_in_stock }
    end

    context 'when no dosage_option but inventory.current_supply is blank' do
      before do
        allow(inventory).to receive(:current_supply).and_return(nil)
        allow(resolver).to receive(:tracked_inventory?).and_return(false)
        allow(resolver).to receive(:call).and_return(nil)
      end

      it { is_expected.to be_in_stock }
    end

    context 'when no dosage_option and inventory.current_supply is present but insufficient' do
      before do
        allow(inventory).to receive(:current_supply).and_return(1)
        allow(resolver).to receive(:tracked_inventory?).and_return(false)
        allow(resolver).to receive(:call).and_return(nil)
        allow(MedicationStockConsumption).to receive(:sufficient?).and_return(false)
      end

      it { is_expected.not_to be_in_stock }
    end
  end
end
