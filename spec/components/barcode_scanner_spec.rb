# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::BarcodeScanner, type: :component do
  subject(:rendered) { render_inline(described_class.new(**options)) }

  let(:options) { {} }

  it 'renders the scanner container with correct data attributes' do
    expect(rendered.css('[data-testid="barcode-scanner"]')).to be_present
    expect(rendered.css('[data-controller="barcode-scanner"]')).to be_present
  end

  it 'renders the start and stop buttons' do
    expect(rendered.css('[data-barcode-scanner-target="startButton"]').text).to include('Start Scanner')
    expect(rendered.css('[data-barcode-scanner-target="stopButton"]').text).to include('Stop Scanner')
  end

  it 'renders the stop button as hidden by default' do
    stop_button = rendered.css('[data-barcode-scanner-target="stopButton"]').first
    expect(stop_button['hidden']).to be_present
  end

  it 'renders the scanner region as hidden by default' do
    region = rendered.css('[data-barcode-scanner-target="scannerRegion"]').first
    expect(region['hidden']).to be_present
  end

  it 'renders the status area with live region attributes' do
    status = rendered.css('[data-barcode-scanner-target="status"]').first
    expect(status['role']).to eq('status')
    expect(status['aria-live']).to eq('polite')
  end

  it 'renders the manual barcode input fallback' do
    expect(rendered.css('[data-testid="manual-barcode-input"]')).to be_present
    expect(rendered.css('[data-barcode-scanner-target="manualInput"]')).to be_present
    expect(rendered.text).to include('Or enter barcode manually')
  end

  it 'renders the manual submit button' do
    expect(rendered.css('[data-action="barcode-scanner#submitManual"]')).to be_present
  end

  context 'with custom formats' do
    let(:options) { { formats: %w[EAN_13 QR_CODE] } }

    it 'passes formats as a data value' do
      container = rendered.css('[data-barcode-scanner-formats-value]').first
      expect(container['data-barcode-scanner-formats-value']).to include('EAN_13')
      expect(container['data-barcode-scanner-formats-value']).to include('QR_CODE')
    end
  end

  context 'with default formats' do
    it 'includes standard barcode formats' do
      container = rendered.css('[data-barcode-scanner-formats-value]').first
      value = container['data-barcode-scanner-formats-value']
      expect(value).to include('EAN_13')
      expect(value).to include('CODE_128')
    end
  end
end
