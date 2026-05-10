# frozen_string_literal: true

require "rails_helper"

RSpec.describe AiMedication::SuggestionValidator do
  let(:allowlist) { instance_double(AiMedication::TrustedSourceAllowlist) }
  let(:validator) { described_class.new(allowlist: allowlist) }

  before do
    allow(allowlist).to(receive(:allowed?).with(calpol_sixplus_url).and_return(true))
    allow(allowlist).to(receive(:allowed?).with(disallowed_url).and_return(false))
  end

  it "drops source links that are not allowlisted" do
    suggestion = validator.call(
      AiMedication::Suggestion.new(
        sources: [
          {url: calpol_sixplus_url, title: "CALPOL SixPlus"},
          {url: disallowed_url, title: "Lookalike source"}
        ]
      )
    )

    expect(suggestion.sources).to(
      contain_exactly(
        include("url" => calpol_sixplus_url, "title" => "CALPOL SixPlus")
      )
    )
  end

  it "drops dose suggestions with evidence from non-allowlisted URLs" do
    suggestion = validator.call(
      AiMedication::Suggestion.new(
        doses: [
          valid_dose,
          valid_dose.merge(evidence: valid_dose[:evidence].merge(url: disallowed_url))
        ]
      )
    )

    expect(suggestion.doses).to(contain_exactly(include("amount" => 5, "unit" => "ml")))
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

  def disallowed_url
    "https://calpol.example.com/unsafe-dose-guidance"
  end
end
