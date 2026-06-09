# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::SourcePage do
  it 'is a Data class with url, title, and text members' do
    page = described_class.new(
      url: 'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol',
      title: 'CALPOL SixPlus',
      text: 'Children 6-8 years 5ml Up to 4 times in 24 hours'
    )

    expect(page.url).to eq('https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol')
    expect(page.title).to eq('CALPOL SixPlus')
    expect(page.text).to eq('Children 6-8 years 5ml Up to 4 times in 24 hours')
  end

  it 'is frozen (Data instances are immutable)' do
    page = described_class.new(url: 'https://example.com', title: 'Example', text: 'Some text')

    expect(page).to be_frozen
  end

  it 'raises when a required member is missing' do
    expect { described_class.new(url: 'https://example.com', title: 'Example') }
      .to raise_error(ArgumentError)
  end

  it 'supports equality by value' do
    page_a = described_class.new(url: 'https://example.com', title: 'Example', text: 'Text')
    page_b = described_class.new(url: 'https://example.com', title: 'Example', text: 'Text')

    expect(page_a).to eq(page_b)
  end

  it 'is not equal when values differ' do
    page_a = described_class.new(url: 'https://example.com', title: 'Example', text: 'Text A')
    page_b = described_class.new(url: 'https://example.com', title: 'Example', text: 'Text B')

    expect(page_a).not_to eq(page_b)
  end
end
