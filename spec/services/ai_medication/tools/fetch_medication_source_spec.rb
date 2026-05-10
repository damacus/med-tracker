# frozen_string_literal: true

require "rails_helper"

RSpec.describe AiMedication::Tools::FetchMedicationSource do
  let(:allowlist) { instance_double(AiMedication::TrustedSourceAllowlist) }
  let(:client) { instance_double(AiMedication::SourcePageClient) }

  it "rejects disallowed URLs before fetching" do
    allow(allowlist).to(receive(:allowed?).with("https://example.com/unsafe").and_return(false))
    allow(client).to(receive(:fetch))

    result = described_class.new(allowlist: allowlist, client: client).execute(url: "https://example.com/unsafe")

    expect(result).to(include(error: "source_not_allowed"))
    expect(client).not_to(have_received(:fetch))
  end

  it "fetches allowed URLs" do
    page = AiMedication::SourcePage.new(
      url: "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol",
      title: "CALPOL SixPlus",
      text: "Children 6-8 years 5ml Up to 4 times in 24 hours"
    )
    allow(allowlist).to(receive(:allowed?).with(page.url).and_return(true))
    allow(client).to(receive(:fetch).with(page.url).and_return(page))

    result = described_class.new(allowlist: allowlist, client: client).execute(url: page.url)

    expect(result).to(include(url: page.url, title: "CALPOL SixPlus"))
    expect(result[:text]).to(include("Children 6-8 years"))
  end
end
