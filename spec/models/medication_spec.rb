# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Medication do
  subject(:medication) do
    described_class.new(
      name: 'Ibuprofen',
      current_supply: 200,
      reorder_threshold: 50
    )
  end

  def blank_dosage_row_attributes
    {
      amount: '',
      unit: 'ml',
      frequency: '',
      description: '',
      default_max_daily_doses: '',
      default_min_hours_between_doses: '',
      default_dose_cycle: 'daily',
      default_for_adults: '0',
      default_for_children: '0',
      _destroy: '0'
    }
  end

  def create_persisted_dose_option(medication)
    medication.dosage_records.create!(
      amount: 2.5,
      unit: 'ml',
      frequency: 'Every morning',
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:name) }
    it { is_expected.not_to validate_presence_of(:current_supply) }
    it { is_expected.to allow_value('sachet').for(:dosage_unit) }
    it { is_expected.to allow_value('capsule').for(:dosage_unit) }
    it { is_expected.to allow_value('pad').for(:dosage_unit) }
    it { is_expected.to validate_numericality_of(:dosage_amount).is_greater_than(0).allow_nil }

    it do
      expect(medication).to validate_numericality_of(:current_supply)
        .only_integer
        .is_greater_than_or_equal_to(0)
        .allow_nil
    end

    it { is_expected.to validate_numericality_of(:reorder_threshold).only_integer.is_greater_than_or_equal_to(0) }

    it { is_expected.to allow_value('Analgesic').for(:category) }
    it { is_expected.to allow_value('Osmotic Laxative').for(:category) }
    it { is_expected.to allow_value('Vitamin').for(:category) }
    it { is_expected.to allow_value(nil).for(:category) }
    it { is_expected.to allow_value('').for(:category) }
    it { is_expected.not_to allow_value('invalid_category').for(:category) }

    it { is_expected.to allow_value(nil).for(:barcode) }
    it { is_expected.to allow_value('').for(:barcode) }
    it { is_expected.to allow_value('5000158100138').for(:barcode) }
    it { is_expected.to allow_value('05000158100138').for(:barcode) }
    it { is_expected.to allow_value(nil).for(:dmd_code) }
    it { is_expected.to allow_value('').for(:dmd_code) }

    it 'rejects non-GTIN barcodes' do
      medication.barcode = '1234567890'
      medication.valid?

      expect(medication.errors[:barcode]).to include('must be a 13 or 14 digit GTIN')
    end

    it 'rejects non-numeric barcodes' do
      medication.barcode = 'abc123'
      medication.valid?

      expect(medication.errors[:barcode]).to include('must be a 13 or 14 digit GTIN')
    end

    it 'rejects duplicate barcodes' do
      create(:medication, barcode: '5000158100138')
      medication.barcode = '5000158100138'
      medication.valid?
      expect(medication.errors[:barcode]).to include('is already linked to another medication in inventory')
    end

    it 'allows multiple nil barcodes' do
      create(:medication, barcode: nil)
      other = build(:medication, barcode: nil)
      expect(other).to be_valid
    end

    it 'requires a dm+d system when a dm+d code is present' do
      medication.dmd_code = '4585411000001109'
      medication.dmd_system = ''
      medication.valid?

      expect(medication.errors[:dmd_system]).to include("can't be blank")
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:location) }
    it { is_expected.to have_many(:dosage_records).class_name('MedicationDosageOption').dependent(:destroy) }
    it { is_expected.to have_many(:schedules).dependent(:destroy) }
  end

  describe 'nested dosage records' do
    it 'ignores untouched auto-appended dose option rows on update' do
      medication = create(:medication, dosage_unit: 'ml')
      create_persisted_dose_option(medication)

      expect(
        medication.update(
          name: 'Updated medication',
          dosage_records_attributes: { '0' => blank_dosage_row_attributes }
        )
      ).to be(true)

      expect(medication.reload.dosage_records.count).to eq(1)
    end
  end

  describe '#restock!' do
    let(:medication) { create(:medication, current_supply: 10, reorder_threshold: 5) }

    it 'increments current_supply by the given quantity' do
      expect { medication.restock!(quantity: 20) }.to change { medication.reload.current_supply }.from(10).to(30)
    end

    it 'sets supply_at_last_restock to the new current_supply' do
      medication.restock!(quantity: 20)
      expect(medication.reload.supply_at_last_restock).to eq(30)
    end

    it 'returns false for zero quantity' do
      expect(medication.restock!(quantity: 0)).to be(false)
    end

    it 'returns false for negative quantity' do
      expect(medication.restock!(quantity: -5)).to be(false)
    end
  end

  describe '#supply_percentage' do
    context 'when supply_at_last_restock is set' do
      let(:medication) { build(:medication, current_supply: 40, supply_at_last_restock: 80, reorder_threshold: 10) }

      it 'returns percentage of current_supply relative to supply_at_last_restock' do
        expect(medication.supply_percentage).to eq(50)
      end
    end

    context 'when supply_at_last_restock is nil' do
      let(:medication) { build(:medication, current_supply: 40, supply_at_last_restock: nil, reorder_threshold: 10) }

      it 'falls back to reorder_threshold as denominator' do
        expect(medication.supply_percentage).to eq(100)
      end
    end

    context 'when current_supply exceeds supply_at_last_restock' do
      let(:medication) { build(:medication, current_supply: 100, supply_at_last_restock: 80, reorder_threshold: 10) }

      it 'caps at 100' do
        expect(medication.supply_percentage).to eq(100)
      end
    end

    context 'when current_supply is zero' do
      let(:medication) { build(:medication, current_supply: 0, supply_at_last_restock: 80, reorder_threshold: 10) }

      it 'returns zero' do
        expect(medication.supply_percentage).to eq(0)
      end
    end

    context 'when current_supply is nil' do
      let(:medication) { build(:medication, current_supply: nil, supply_at_last_restock: nil, reorder_threshold: 10) }

      it 'returns zero' do
        expect(medication.supply_percentage).to eq(0)
      end
    end
  end

  describe '#low_stock?' do
    subject(:medication) do
      described_class.new(
        name: 'Ibuprofen',
        current_supply: current_supply,
        reorder_threshold: 50
      )
    end

    context 'when current_supply is below the reorder threshold' do
      let(:current_supply) { 25 }

      it 'returns true' do
        expect(medication.low_stock?).to be(true)
      end
    end

    context 'when current_supply meets the reorder threshold' do
      let(:current_supply) { 50 }

      it 'returns true' do
        expect(medication.low_stock?).to be(true)
      end
    end

    context 'when current_supply is above the reorder threshold' do
      let(:current_supply) { 75 }

      it 'returns false' do
        expect(medication.low_stock?).to be(false)
      end
    end
  end

  describe '#out_of_stock?' do
    subject(:medication) { described_class.new(current_supply: current_supply) }

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

  describe '#estimated_daily_consumption' do
    let(:medication) { create(:medication, current_supply: 100, reorder_threshold: 10) }

    context 'with no schedules or person_medications' do
      it 'returns 0.0 with no schedules or person_medications' do
        expect(medication.estimated_daily_consumption).to eq(0.0)
      end
    end

    context 'with a daily schedule' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 4, dose_cycle: :daily)
      end

      it 'returns the max_daily_doses' do
        expect(medication.estimated_daily_consumption).to eq(4.0)
      end
    end

    context 'with a weekly schedule' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, :weekly, medication: medication, dosage: dosage, max_daily_doses: 7)
      end

      it 'normalizes weekly schedule to daily rate' do
        expect(medication.estimated_daily_consumption).to eq(1.0)
      end
    end

    context 'with a monthly schedule' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, :monthly, medication: medication, dosage: dosage, max_daily_doses: 30)
      end

      it 'normalizes monthly schedule to daily rate' do
        expect(medication.estimated_daily_consumption).to eq(1.0)
      end
    end

    context 'with multiple schedules' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 4, dose_cycle: :daily)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 2, dose_cycle: :daily)
      end

      it 'sums across all active schedules' do
        expect(medication.estimated_daily_consumption).to eq(6.0)
      end
    end

    context 'with expired schedules' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, :expired, medication: medication, dosage: dosage, max_daily_doses: 4)
      end

      it 'excludes expired schedules' do
        expect(medication.estimated_daily_consumption).to eq(0.0)
      end
    end

    context 'with person_medications' do
      before do
        create(:person_medication, medication: medication, max_daily_doses: 2)
      end

      it 'includes person_medication daily doses' do
        expect(medication.estimated_daily_consumption).to eq(2.0)
      end
    end

    context 'with schedules and person_medications combined' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 3, dose_cycle: :daily)
        create(:person_medication, medication: medication, max_daily_doses: 1)
      end

      it 'sums both sources' do
        expect(medication.estimated_daily_consumption).to eq(4.0)
      end
    end

    context 'with nil max_daily_doses' do
      before do
        create(:person_medication, medication: medication, max_daily_doses: nil)
      end

      it 'treats nil as zero' do
        expect(medication.estimated_daily_consumption).to eq(0.0)
      end
    end
  end

  describe '#forecast_available?' do
    let(:medication) { create(:medication, current_supply: 100, reorder_threshold: 10) }

    context 'when current_supply is present and daily consumption is positive' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 2, dose_cycle: :daily)
      end

      it 'returns true' do
        expect(medication.forecast_available?).to be(true)
      end
    end

    context 'when current_supply is nil' do
      let(:medication) { create(:medication, current_supply: nil) }

      it 'returns false' do
        expect(medication.forecast_available?).to be(false)
      end
    end

    context 'when no schedules exist' do
      it 'returns false' do
        expect(medication.forecast_available?).to be(false)
      end
    end
  end

  describe '#days_until_out_of_stock' do
    let(:medication) { create(:medication, current_supply: 20, reorder_threshold: 5) }

    context 'when forecast is available' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 4, dose_cycle: :daily)
      end

      it 'returns the number of days' do
        expect(medication.days_until_out_of_stock).to eq(5)
      end
    end

    context 'when already out of stock' do
      let(:medication) { create(:medication, current_supply: 0, reorder_threshold: 5) }

      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 4, dose_cycle: :daily)
      end

      it 'returns zero' do
        expect(medication.days_until_out_of_stock).to eq(0)
      end
    end

    context 'when forecast is not available' do
      it 'returns nil' do
        expect(medication.days_until_out_of_stock).to be_nil
      end
    end

    context 'with fractional daily rate' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, :weekly, medication: medication, dosage: dosage, max_daily_doses: 7)
      end

      it 'rounds up to nearest day' do
        expect(medication.days_until_out_of_stock).to eq(20)
      end
    end
  end

  describe '#days_until_low_stock' do
    let(:medication) { create(:medication, current_supply: 20, reorder_threshold: 5) }

    context 'when forecast is available' do
      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 5, dose_cycle: :daily)
      end

      it 'returns the number of days until supply reaches threshold' do
        expect(medication.days_until_low_stock).to eq(3)
      end
    end

    context 'when already low stock' do
      let(:medication) { create(:medication, current_supply: 5, reorder_threshold: 10) }

      before do
        dosage = create(:dosage, medication: medication)
        create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 2, dose_cycle: :daily)
      end

      it 'returns zero' do
        expect(medication.days_until_low_stock).to eq(0)
      end
    end

    context 'when forecast is not available' do
      it 'returns nil' do
        expect(medication.days_until_low_stock).to be_nil
      end
    end
  end

  describe '#out_of_stock_date' do
    let(:medication) { create(:medication, current_supply: 20, reorder_threshold: 5) }

    before do
      dosage = create(:dosage, medication: medication)
      create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 4, dose_cycle: :daily)
    end

    it 'returns the forecasted date' do
      expect(medication.out_of_stock_date).to eq(Time.zone.today + 5.days)
    end
  end

  describe '#low_stock_date' do
    let(:medication) { create(:medication, current_supply: 25, reorder_threshold: 5) }

    before do
      dosage = create(:dosage, medication: medication)
      create(:schedule, medication: medication, dosage: dosage, max_daily_doses: 5, dose_cycle: :daily)
    end

    it 'returns the forecasted date' do
      expect(medication.low_stock_date).to eq(Time.zone.today + 4.days)
    end
  end

  describe '#sync_dosages' do
    let(:medication) { create(:medication, dosage_amount: nil, dosage_unit: nil) }

    before do
      create(:dosage, medication: medication, amount: 10, unit: 'mg')
      create(:dosage, medication: medication, amount: 20, unit: 'mg')
    end

    context 'when switching from multi-dose to single-dose' do
      it 'removes associated dosages when dosage_amount is set and there are no schedules' do
        expect do
          medication.update!(dosage_amount: 500, dosage_unit: 'mg')
        end.to change { medication.dosages.count }.from(2).to(0)
      end
    end

    context 'when schedules still use the dosage options' do
      before do
        create(:schedule, medication: medication, dosage: medication.dosages.first)
      end

      it 'rejects the switch to single-dose mode' do
        expect(medication.update(dosage_amount: 500, dosage_unit: 'mg')).to be(false)
        expect(medication.errors[:dosage_amount]).to include(
          'cannot switch to a single standard dose while schedules still use dose options'
        )
      end

      it 'keeps the existing dosages intact' do
        expect do
          medication.update(dosage_amount: 500, dosage_unit: 'mg')
        end.not_to(change { medication.dosages.count })
      end
    end

    context 'when remaining in multi-dose mode' do
      it 'does not remove dosages when dosage_amount remains nil' do
        expect do
          medication.update!(name: 'New Name')
        end.not_to change(medication.dosages, :count)
      end
    end
  end
end
