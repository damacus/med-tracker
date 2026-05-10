# frozen_string_literal: true

require "rails_helper"

RSpec.describe AiMedication::AuditLogger do
  subject(:audit_logger) { described_class.new }

  fixtures(:accounts, :people, :users)

  let(:user) { users(:admin) }
  let(:medication_identity) { {name: "Calpol Six Plus"} }
  let(:found_suggestion) do
    AiMedication::Suggestion.new(
      sources: [{url: "https://example.com", title: "Example"}],
      doses: [{amount: 5, unit: "ml"}]
    )
  end

  let(:error_suggestion) { AiMedication::Suggestion.new(errors: ["suggestion_unavailable"]) }

  before do
    PaperTrail.request.controller_info = {ip: "10.0.0.1", request_id: "req-ai-001"}
  end

  after do
    PaperTrail.request.controller_info = {}
  end

  describe "#record" do
    it "creates a PaperTrail::Version with item_type AiMedicationSuggestion" do
      expect do
        audit_logger.record(user: user, medication_identity: medication_identity, suggestion: found_suggestion)
      end
        .to(change { PaperTrail::Version.where(item_type: "AiMedicationSuggestion").count }.by(1))
    end

    it "persists the correct event, whodunnit, ip, and request_id" do
      audit_logger.record(user: user, medication_identity: medication_identity, suggestion: found_suggestion)

      version = PaperTrail::Version.where(item_type: "AiMedicationSuggestion").last
      expect(version.event).to(eq("ai_medication/suggestion"))
      expect(version.whodunnit).to(eq(user.id.to_s))
      expect(version.ip).to(eq("10.0.0.1"))
      expect(version.request_id).to(eq("req-ai-001"))
    end

    it "stores a SHA256 identity hash and counts in object JSON" do
      audit_logger.record(user: user, medication_identity: medication_identity, suggestion: found_suggestion)

      data = JSON.parse(PaperTrail::Version.where(item_type: "AiMedicationSuggestion").last.object)
      expect(data["identity_hash"]).to(eq(Digest::SHA256.hexdigest(medication_identity.to_s.strip.downcase)))
      expect(data["source_count"]).to(eq(1))
      expect(data["dose_count"]).to(eq(1))
      expect(data["error_count"]).to(eq(0))
      expect(data["result_status"]).to(eq("found"))
    end

    it "records result_status as error when suggestion has errors" do
      audit_logger.record(user: user, medication_identity: medication_identity, suggestion: error_suggestion)

      data = JSON.parse(PaperTrail::Version.where(item_type: "AiMedicationSuggestion").last.object)
      expect(data["result_status"]).to(eq("error"))
      expect(data["error_count"]).to(eq(1))
    end

    it "does not raise when called without controller context" do
      PaperTrail.request.controller_info = {}

      expect do
        audit_logger.record(user: nil, medication_identity: medication_identity, suggestion: found_suggestion)
      end
        .not_to(raise_error)
    end

    it "silently rescues errors and logs them" do
      allow(PaperTrail::Version).to(receive(:insert).and_raise(ActiveRecord::StatementInvalid))
      allow(Rails.logger).to(receive(:error))

      expect do
        audit_logger.record(user: user, medication_identity: medication_identity, suggestion: found_suggestion)
      end
        .not_to(raise_error)

      expect(Rails.logger).to(have_received(:error).with(/AiMedication::AuditLogger failed/))
    end
  end
end
