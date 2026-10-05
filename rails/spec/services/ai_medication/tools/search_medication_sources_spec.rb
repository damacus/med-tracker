# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::Tools::SearchMedicationSources do
  subject(:tool) { described_class.new(allowlist: allowlist) }

  let(:allowlist) { instance_double(AiMedication::TrustedSourceAllowlist) }
  let(:calpol_url) { 'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol' }
  let(:medicines_url) { 'https://www.medicines.org.uk/emc/product/13866/pil' }
  let(:calpol_source) do
    { 'url' => calpol_url, 'title' => 'CALPOL SixPlus', 'keywords' => %w[calpol paracetamol sixplus] }
  end
  let(:medicines_source) do
    { 'url' => medicines_url, 'title' => 'Medicines.org.uk — Ibuprofen', 'keywords' => %w[ibuprofen nurofen] }
  end

  before do
    allow(allowlist).to receive(:seed_urls).and_return([calpol_source, medicines_source])
    allow(allowlist).to receive(:allowed?).with(calpol_url).and_return(true)
    allow(allowlist).to receive(:allowed?).with(medicines_url).and_return(true)
  end

  describe '#execute' do
    it 'returns matching sources for a relevant query' do
      results = tool.execute(query: 'calpol paracetamol')

      expect(results).to contain_exactly(
        include(url: calpol_url, title: 'CALPOL SixPlus')
      )
    end

    it 'returns empty array when no sources match the query' do
      results = tool.execute(query: 'completely unrelated xyz')

      expect(results).to be_empty
    end

    it 'includes matched_keywords in each result' do
      results = tool.execute(query: 'paracetamol children')

      expect(results.first[:matched_keywords]).to include('paracetamol')
    end

    it 'excludes sources that are not allowed by the allowlist' do
      allow(allowlist).to receive(:allowed?).with(calpol_url).and_return(false)

      results = tool.execute(query: 'calpol paracetamol')

      expect(results).to be_empty
    end

    it 'matches keywords case-insensitively' do
      results = tool.execute(query: 'CALPOL')

      expect(results).not_to be_empty
      expect(results.first[:url]).to eq(calpol_url)
    end

    it 'ignores very short tokens (under 3 characters)' do
      # "of" is 2 chars and should be ignored; "sixplus" should still match
      results = tool.execute(query: 'of sixplus')

      expect(results).not_to be_empty
    end

    it 'returns multiple matches when the query spans multiple sources' do
      results = tool.execute(query: 'paracetamol ibuprofen')

      expect(results.pluck(:url)).to contain_exactly(calpol_url, medicines_url)
    end

    it 'returns the url, title, and matched_keywords keys in each result' do
      results = tool.execute(query: 'calpol')

      result = results.first
      expect(result).to have_key(:url)
      expect(result).to have_key(:title)
      expect(result).to have_key(:matched_keywords)
    end
  end
end
