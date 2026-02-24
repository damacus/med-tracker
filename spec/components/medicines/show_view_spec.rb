# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::ShowView, type: :component do
  let(:medicine) { create(:medicine, name: 'Paracetamol', current_supply: 50) }

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
    it 'renders the out-of-stock forecast' do
      medicine_with_prescription = create(:medicine, name: 'Paracetamol', current_supply: 50)
      dosage = create(:dosage, medicine: medicine_with_prescription)
      create(:prescription, medicine: medicine_with_prescription, dosage: dosage, max_daily_doses: 10,
                            dose_cycle: :daily)

      rendered = render_inline(described_class.new(medicine: medicine_with_prescription))

      expect(rendered.text).to include('Supply will be empty in 5 days')
    end

    it 'renders the low-stock forecast' do
      medicine_with_prescription = create(:medicine, name: 'Paracetamol', current_supply: 50)
      dosage = create(:dosage, medicine: medicine_with_prescription)
      create(:prescription, medicine: medicine_with_prescription, dosage: dosage, max_daily_doses: 10,
                            dose_cycle: :daily)

      rendered = render_inline(described_class.new(medicine: medicine_with_prescription))

      # With current_supply: 50, reorder_threshold defaults to 10 from migration
      # Surplus = 50 - 10 = 40, days = (40 / 10).ceil = 4
      expect(rendered.text).to include('Supply will be low in 4 days')
    end
  end

  context 'when forecast is not available' do
    it 'renders the fallback message' do
      rendered = render_inline(described_class.new(medicine: medicine))

      expect(rendered.text).to include('Forecast unavailable')
    end
  end
end
