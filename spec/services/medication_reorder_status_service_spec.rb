# frozen_string_literal: true

require "rails_helper"

RSpec.describe MedicationReorderStatusService do
  describe "#call" do
    let(:medication) { create(:medication, reorder_status: nil, ordered_at: nil, reordered_at: nil) }

    it "marks a medication as ordered" do
      timestamp = Time.zone.local(2026, 5, 5, 9, 30)

      result = described_class.new.call(medication: medication, status: :ordered, at: timestamp)

      expect(result).to(be_success)
      expect(medication.reload).to(
        have_attributes(
          reorder_status: "ordered",
          ordered_at: timestamp
        )
      )
    end

    it "creates a PaperTrail version with event mark_as_ordered" do
      expect do
        described_class.new.call(medication: medication, status: :ordered)
      end
        .to(change { PaperTrail::Version.where(item_type: "Medication", item_id: medication.id).count }.by(1))

      expect(PaperTrail::Version.where(item_type: "Medication", item_id: medication.id).last.event)
        .to(eq("mark_as_ordered"))
    end

    it "marks a medication as received" do
      timestamp = Time.zone.local(2026, 5, 5, 10, 30)

      result = described_class.new.call(medication: medication, status: :received, at: timestamp)

      expect(result).to(be_success)
      expect(medication.reload).to(
        have_attributes(
          reorder_status: "received",
          reordered_at: timestamp
        )
      )
    end

    it "creates a PaperTrail version with event mark_as_received" do
      expect do
        described_class.new.call(medication: medication, status: :received)
      end
        .to(change { PaperTrail::Version.where(item_type: "Medication", item_id: medication.id).count }.by(1))

      expect(PaperTrail::Version.where(item_type: "Medication", item_id: medication.id).last.event)
        .to(eq("mark_as_received"))
    end

    it "returns false for unsupported statuses" do
      result = described_class.new.call(medication: medication, status: :cancelled)

      expect(result).not_to(be_success)
      expect(medication.reload.reorder_status).to(be_nil)
    end
  end
end
