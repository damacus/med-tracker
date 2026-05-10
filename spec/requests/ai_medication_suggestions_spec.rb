# frozen_string_literal: true

require "rails_helper"

RSpec.describe "AI medication suggestions" do
  fixtures(:accounts, :people, :users, :locations, :location_memberships)

  before { sign_in(users(:admin)) }

  it "is unavailable when the environment flag is disabled" do
    users(:admin).person.account.update!(subscription_plan: "family_plus")
    allow(ENV).to(receive(:fetch).with("MEDTRACKER_AI_MEDICATION_HELP_ENABLED", "false").and_return("false"))

    post(ai_medication_suggestions_path, params: {medication: {name: "Calpol Six Plus"}})

    expect(response).to(have_http_status(:not_found))
  end

  it "is unavailable when the account plan is not entitled" do
    allow(ENV).to(receive(:fetch).with("MEDTRACKER_AI_MEDICATION_HELP_ENABLED", "false").and_return("true"))

    post(ai_medication_suggestions_path, params: {medication: {name: "Calpol Six Plus"}})

    expect(response).to(have_http_status(:not_found))
  end

  it "returns source-linked draft suggestions when enabled" do
    suggestion = AiMedication::Suggestion.new(
      medication: {description: "Paracetamol pain and fever relief", warnings: "Contains paracetamol"},
      doses: [
        {
          amount: 5,
          unit: "ml",
          description: "Children 6-8 years",
          default_max_daily_doses: 4,
          default_min_hours_between_doses: 4,
          default_dose_cycle: "daily",
          evidence: {
            url: "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol",
            title: "CALPOL SixPlus",
            text: "Children 6-8 years 5ml Up to 4 times in 24 hours"
          }
        }
      ],
      sources: [
        {
          url: "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol",
          title: "CALPOL SixPlus"
        }
      ]
    )
    service = instance_double(AiMedication::SuggestionService, call: suggestion)

    users(:admin).person.account.update!(subscription_plan: "family_plus")
    allow(ENV).to(receive(:fetch).with("MEDTRACKER_AI_MEDICATION_HELP_ENABLED", "false").and_return("true"))
    allow(AiMedication::SuggestionService).to(receive(:new).and_return(service))

    post(ai_medication_suggestions_path, params: {medication: {name: "Calpol Six Plus"}})

    expect(response).to(have_http_status(:ok))
    expect(response.parsed_body.dig("doses", 0, "evidence", "url")).to(
      eq(
        "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol"
      )
    )
  end
end
