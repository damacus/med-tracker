# frozen_string_literal: true

require "rails_helper"

RSpec.describe MedicationStockQuantityFormatter do
  describe ".format" do
    it "formats whole quantities without a decimal point" do
      expect(described_class.format(BigDecimal("10.0"))).to(eq("10"))
    end

    it "preserves meaningful decimal quantities" do
      expect(described_class.format(BigDecimal("97.5"))).to(eq("97.5"))
    end

    it "strips trailing zeroes from decimal quantities" do
      expect(described_class.format(BigDecimal("12.50"))).to(eq("12.5"))
    end
  end
end
