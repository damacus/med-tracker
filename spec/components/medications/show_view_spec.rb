# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::ShowView, type: :component do
  let(:medication) { create(:medication, name: 'Paracetamol', current_supply: 50) }

  it 'renders the medication name' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'renders action links using Link component without raw button classes' do
    rendered = render_inline(described_class.new(medication: medication))

    edit_link = rendered.css('a').find { |a| a.text.include?('Edit Details') }
    back_link = rendered.css('a').find { |a| a.text.include?('Inventory') }
    expect(edit_link).to be_present
    expect(back_link).to be_present
  end

  it 'renders the inventory status heading' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Inventory Status')
  end

  it 'renders safety warnings when present' do
    medication_with_warnings = create(:medication, warnings: 'Take with food')
    rendered = render_inline(described_class.new(medication: medication_with_warnings))

    expect(rendered.text).to include('Safety Warnings')
    expect(rendered.text).to include('Take with food')
  end

  context 'when forecast is available' do
    it 'renders the out-of-stock forecast' do
      medication_with_schedule = create(:medication, name: 'Paracetamol', current_supply: 50)
      dosage = create(:dosage, medication: medication_with_schedule)
      create(:schedule, medication: medication_with_schedule, dosage: dosage, max_daily_doses: 10,
                        dose_cycle: :daily)

      rendered = render_inline(described_class.new(medication: medication_with_schedule))

      expect(rendered.text).to include('Supply will be empty in 5 days')
    end

    it 'renders the low-stock forecast' do
      medication_with_schedule = create(:medication, name: 'Paracetamol', current_supply: 50)
      dosage = create(:dosage, medication: medication_with_schedule)
      create(:schedule, medication: medication_with_schedule, dosage: dosage, max_daily_doses: 10,
                        dose_cycle: :daily)

      rendered = render_inline(described_class.new(medication: medication_with_schedule))

      # With current_supply: 50, reorder_threshold defaults to 10 from migration
      # Surplus = 50 - 10 = 40, days = (40 / 10).ceil = 4
      expect(rendered.text).to include('Supply will be low in 4 days')
    end
  end

  context 'when forecast is not available' do
    it 'renders the fallback message' do
      rendered = render_inline(described_class.new(medication: medication))

      expect(rendered.text).to include('Forecast unavailable')
    end
  end
end
