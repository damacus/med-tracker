# frozen_string_literal: true

require "rails_helper"

RSpec.describe MedicationTakeStockDecrement do
  subject(:decrementer) { described_class.new(take) }

  let(:take) { instance_double(MedicationTake, dose_amount: 10, dose_unit: "mg") }

  describe "#call" do
    it "serializes stale aggregate inventory decrements from separate sessions" do
      medication = create(:medication, current_supply: 2, reorder_threshold: 0)

      decrement_stale_inventory(medication)
      decrement_stale_inventory(medication)

      expect(medication.reload.current_supply).to(eq(BigDecimal("0")))
    end

    it "creates a dose_decrement PaperTrail version on the medication" do
      medication = create(:medication, current_supply: 10, reorder_threshold: 0)

      expect do
        decrementer.call(stock_source_for(inventory: stale_medication(medication)))
      end
        .to(change { PaperTrail::Version.where(item_type: "Medication", item_id: medication.id).count }.by(1))

      expect(PaperTrail::Version.where(item_type: "Medication", item_id: medication.id).last.event)
        .to(eq("dose_decrement"))
    end

    it "does not allow stale aggregate inventory decrements to push supply below zero" do
      medication = create(:medication, current_supply: 1, reorder_threshold: 0)

      decrement_stale_inventory(medication)
      decrement_stale_inventory(medication)

      expect(medication.reload.current_supply).to(eq(BigDecimal("0")))
    end

    it "serializes stale dosage-option decrements and keeps aggregate inventory in sync" do
      medication = create(:medication, current_supply: 2, reorder_threshold: 0)
      dosage_option = create(
        :dosage,
        medication: medication,
        amount: 1,
        unit: "tablet",
        current_supply: 2,
        reorder_threshold: 0
      )

      decrement_stale_dosage_option(medication: medication, dosage_option: dosage_option)
      decrement_stale_dosage_option(medication: medication, dosage_option: dosage_option)

      expect(dosage_option.reload.current_supply).to(eq(BigDecimal("0")))
      expect(medication.reload.current_supply).to(eq(BigDecimal("0")))
    end
  end

  def stock_source_for(inventory:, dosage_option: nil)
    instance_double(MedicationTakeStockSource, inventory: inventory, dosage_option: dosage_option)
  end

  def decrement_stale_inventory(medication)
    decrementer.call(stock_source_for(inventory: stale_medication(medication)))
  end

  def decrement_stale_dosage_option(medication:, dosage_option:)
    decrementer.call(
      stock_source_for(
        inventory: stale_medication(medication),
        dosage_option: stale_dosage_option(dosage_option)
      )
    )
  end

  def stale_medication(medication)
    Medication.find(medication.id)
  end

  def stale_dosage_option(dosage_option)
    MedicationDosageOption.find(dosage_option.id)
  end
end
