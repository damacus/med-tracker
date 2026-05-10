# frozen_string_literal: true

require "rails_helper"

RSpec.describe AiMedication::SuggestionService do
  fixtures(:accounts, :people, :users)

  it "keeps valid source-linked dose suggestions" do
    suggestion = call_service(valid_suggestion)

    expect(suggestion.doses.first).to(include("amount" => 5, "unit" => "ml"))
    expect(suggestion.doses.first.dig("evidence", "url")).to(eq(calpol_sixplus_url))
  end

  it "drops invalid dose suggestions before rendering" do
    suggestion = call_service(invalid_suggestion)

    expect(suggestion.doses).to(be_empty)
  end

  it "records an audit entry on success" do
    audit_logger = instance_double(AiMedication::AuditLogger, record: true)
    assistant = instance_double(AiMedication::RubyLlmAssistant, call: valid_suggestion)

    described_class.new(assistant: assistant, audit_logger: audit_logger).call(
      medication_identity: {name: "Calpol Six Plus"},
      user: users(:admin)
    )

    expect(audit_logger).to(
      have_received(:record).with(
        user: users(:admin),
        medication_identity: {name: "Calpol Six Plus"},
        suggestion: instance_of(AiMedication::Suggestion)
      )
    )
  end

  it "records an audit entry when the assistant raises" do
    audit_logger = instance_double(AiMedication::AuditLogger, record: true)
    assistant = instance_double(AiMedication::RubyLlmAssistant)
    allow(assistant).to(receive(:call).and_raise(StandardError, "LLM unavailable"))

    described_class.new(assistant: assistant, audit_logger: audit_logger).call(
      medication_identity: {name: "Calpol Six Plus"},
      user: users(:admin)
    )

    expect(audit_logger).to(
      have_received(:record).with(
        user: users(:admin),
        medication_identity: {name: "Calpol Six Plus"},
        suggestion: have_attributes(errors: ["suggestion_unavailable"])
      )
    )
  end

  it "returns the validated suggestion when audit logging fails" do
    audit_logger = instance_double(AiMedication::AuditLogger)
    assistant = instance_double(AiMedication::RubyLlmAssistant, call: valid_suggestion)
    allow(audit_logger).to(receive(:record).and_raise(StandardError, "audit unavailable"))

    suggestion = described_class.new(assistant: assistant, audit_logger: audit_logger).call(
      medication_identity: {name: "Calpol Six Plus"},
      user: users(:admin)
    )

    expect(suggestion.doses.first).to(include("amount" => 5, "unit" => "ml"))
  end

  it "returns the fallback suggestion when error audit logging fails" do
    audit_logger = instance_double(AiMedication::AuditLogger)
    assistant = instance_double(AiMedication::RubyLlmAssistant)
    allow(assistant).to(receive(:call).and_raise(StandardError, "LLM unavailable"))
    allow(audit_logger).to(receive(:record).and_raise(StandardError, "audit unavailable"))

    suggestion = described_class.new(assistant: assistant, audit_logger: audit_logger).call(
      medication_identity: {name: "Calpol Six Plus"},
      user: users(:admin)
    )

    expect(suggestion.errors).to(eq(["suggestion_unavailable"]))
  end

  def call_service(raw_suggestion)
    assistant = instance_double(AiMedication::RubyLlmAssistant, call: raw_suggestion)
    audit_logger = instance_double(AiMedication::AuditLogger, record: true)

    described_class.new(assistant: assistant, audit_logger: audit_logger).call(
      medication_identity: {name: "Calpol Six Plus"},
      user: users(:admin)
    )
  end

  def valid_suggestion
    AiMedication::Suggestion.new(
      medication: {description: "Paracetamol pain and fever relief"},
      doses: [valid_dose],
      sources: [{url: calpol_sixplus_url, title: "CALPOL SixPlus"}]
    )
  end

  def invalid_suggestion
    AiMedication::Suggestion.new(
      doses: [
        valid_dose.merge(amount: -1),
        valid_dose.except(:evidence)
      ]
    )
  end

  def valid_dose
    {
      amount: 5,
      unit: "ml",
      description: "Children 6-8 years",
      default_max_daily_doses: 4,
      default_min_hours_between_doses: 4,
      default_dose_cycle: "daily",
      evidence: {
        url: calpol_sixplus_url,
        title: "CALPOL SixPlus",
        text: "Children 6-8 years 5ml Up to 4 times in 24 hours"
      }
    }
  end

  def calpol_sixplus_url
    "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol"
  end
end
