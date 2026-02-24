# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::ShowView, type: :component do
  let(:medicine) { create(:medicine, name: 'Paracetamol', current_supply: 50, stock: 100) }

  it 'renders the medicine name' do
    rendered = render_inline(described_class.new(medicine: medicine))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'renders action links using Link component without raw button classes' do
    rendered = render_inline(described_class.new(medicine: medicine))

    edit_link = rendered.css('a').find { |a| a.text.include?('Edit Details') }
    back_link = rendered.css('a').find { |a| a.text.include?('Inventory') }
    expect(edit_link).to be_present
    expect(back_link).to be_present
  end

  it 'renders the inventory status heading' do
    rendered = render_inline(described_class.new(medicine: medicine))

    expect(rendered.text).to include('Inventory Status')
  end

  it 'renders safety warnings when present' do
    medicine_with_warnings = create(:medicine, warnings: 'Take with food')
    rendered = render_inline(described_class.new(medicine: medicine_with_warnings))

    expect(rendered.text).to include('Safety Warnings')
    expect(rendered.text).to include('Take with food')
  end

  context 'when forecast is available' do
    before do
      dosage = create(:dosage, medicine: medicine)
      create(:prescription, medicine: medicine, dosage: dosage, max_daily_doses: 10, dose_cycle: :daily)
    end

    it 'renders the out-of-stock forecast' do
      rendered = render_inline(described_class.new(medicine: medicine))

      expect(rendered.text).to include('Stock will be empty in 5 days')
    end

    it 'renders the low-stock forecast' do
      rendered = render_inline(described_class.new(medicine: medicine))

      expect(rendered.text).to match(/Stock will be low in \d+ days/)
    end
  end

  context 'when forecast is not available' do
    it 'renders the fallback message' do
      rendered = render_inline(described_class.new(medicine: medicine))

      expect(rendered.text).to include('Forecast unavailable')
    end
  end
end
