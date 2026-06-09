# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AiMedication::SourcePageClient do
  subject(:client) { described_class.new }

  let(:valid_url) { 'https://www.calpol.co.uk/our-products/calpol-sixplus' }
  let(:html_body) do
    '<html><head><title>CALPOL SixPlus</title></head><body><p>Children 6-8 years 5ml</p></body></html>'
  end

  describe '#fetch' do
    context 'with a successful response' do
      before do
        stub_request(:get, valid_url)
          .to_return(status: 200, body: html_body, headers: { 'Content-Type' => 'text/html' })
      end

      it 'returns a SourcePage with the correct url' do
        page = client.fetch(valid_url)

        expect(page).to be_a(AiMedication::SourcePage)
        expect(page.url).to eq(valid_url)
      end

      it 'extracts the title from the HTML' do
        page = client.fetch(valid_url)

        expect(page.title).to eq('CALPOL SixPlus')
      end

      it 'strips tags and returns plain text' do
        page = client.fetch(valid_url)

        expect(page.text).to include('Children 6-8 years 5ml')
        expect(page.text).not_to include('<p>')
      end
    end

    context 'when the response is not a success' do
      before do
        stub_request(:get, valid_url).to_return(status: 404)
      end

      it 'raises with the HTTP status code' do
        expect { client.fetch(valid_url) }.to raise_error(RuntimeError, /404/)
      end
    end

    context 'when the URL is invalid' do
      it 'raises a descriptive error' do
        expect { client.fetch('not a url') }.to raise_error(RuntimeError, /Source fetch failed/)
      end
    end

    context 'when a network timeout occurs' do
      before do
        stub_request(:get, valid_url).to_timeout
      end

      it 'raises a descriptive error' do
        expect { client.fetch(valid_url) }.to raise_error(RuntimeError, /Source fetch failed/)
      end
    end

    context 'with a very long HTML body' do
      let(:long_text) { 'word ' * 10_000 }
      let(:long_html) { "<html><head><title>Long Page</title></head><body>#{long_text}</body></html>" }

      before do
        stub_request(:get, valid_url)
          .to_return(status: 200, body: long_html, headers: { 'Content-Type' => 'text/html' })
      end

      it 'truncates text to MAX_TEXT_LENGTH' do
        page = client.fetch(valid_url)

        expect(page.text.length).to be <= described_class::MAX_TEXT_LENGTH
      end
    end

    context 'when the title tag is absent' do
      let(:no_title_html) { '<html><body><p>No title here</p></body></html>' }

      before do
        stub_request(:get, valid_url)
          .to_return(status: 200, body: no_title_html, headers: { 'Content-Type' => 'text/html' })
      end

      it 'returns an empty string for the title' do
        page = client.fetch(valid_url)

        expect(page.title).to eq('')
      end
    end
  end
end
