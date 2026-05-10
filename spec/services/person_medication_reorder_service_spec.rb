# frozen_string_literal: true

require "rails_helper"

RSpec.describe PersonMedicationReorderService do
  describe "#call" do
    let(:person) { create(:person) }
    let(:first) { create(:person_medication, person: person, position: 1) }
    let(:second) { create(:person_medication, person: person, position: 2) }

    it "moves a person medication up by swapping with the previous record" do
      first

      result = described_class.new.call(person_medication: second, direction: "up")

      expect(result).to(be_success)
      expect(person.person_medications.order(:position, :id)).to(eq([second.reload, first.reload]))
    end

    it "moves a person medication down by swapping with the next record" do
      second

      result = described_class.new.call(person_medication: first, direction: "down")

      expect(result).to(be_success)
      expect(person.person_medications.order(:position, :id)).to(eq([second.reload, first.reload]))
    end

    it "returns false when there is no adjacent record" do
      result = described_class.new.call(person_medication: first, direction: "up")

      expect(result).not_to(be_success)
      expect(first.reload.position).to(eq(1))
    end

    it "returns false for an unsupported direction" do
      first

      result = described_class.new.call(person_medication: second, direction: "sideways")

      expect(result).not_to(be_success)
      expect(person.person_medications.order(:position, :id)).to(eq([first.reload, second.reload]))
    end
  end
end
