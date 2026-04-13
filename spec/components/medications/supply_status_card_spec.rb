# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::SupplyStatusCard, type: :component do
  let(:medication) { create(:medication, name: 'Paracetamol', current_supply: 50) }

  it 'renders the inventory status heading' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Inventory Status')
  end

  context 'when forecast is available' do
    it 'renders the out-of-stock forecast' do
      medication_with_schedule = create(:medication, name: 'Paracetamol', current_supply: 50)
      create(
        :schedule,
        medication: medication_with_schedule,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 10,
        dose_cycle: :daily
      )

      rendered = render_inline(described_class.new(medication: medication_with_schedule))

      expect(rendered.text).to include('Supply will be empty in 5 days')
    end

    it 'renders the low-stock forecast' do
      medication_with_schedule = create(:medication, name: 'Paracetamol', current_supply: 50)
      create(
        :schedule,
        medication: medication_with_schedule,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 10,
        dose_cycle: :daily
      )

      rendered = render_inline(described_class.new(medication: medication_with_schedule))

      expect(rendered.text).to include('Supply will be low in 4 days')
    end
  end

  context 'when forecast is not available' do
    it 'renders the fallback message' do
      rendered = render_inline(described_class.new(medication: medication))

      expect(rendered.text).to include('Forecast unavailable')
    end
  end

  context 'when reorder status is present' do
    it 'renders the reorder status badge and timestamp' do
      medication.update!(
        current_supply: 5,
        supply_at_last_restock: 50,
        reorder_status: :ordered,
        ordered_at: 2.hours.ago
      )

      rendered = render_inline(described_class.new(medication: medication))

      expect(rendered.text).to include('Ordered')
      expect(rendered.text).to match(/Ordered .* ago/)
    end
  end
end
