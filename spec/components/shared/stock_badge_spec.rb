# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::StockBadge, type: :component do
  subject(:component) { described_class.new(medication: medication) }

  let(:supply_level) { instance_double(SupplyLevel, status: status) }
  let(:medication) do
    instance_double(Medication, current_supply: current_supply, supply_level: supply_level)
  end
  let(:current_supply) { 100 }
  let(:status) { :in_stock }

  describe '#view_template' do
    context 'when medication has no current_supply value' do
      let(:current_supply) { nil }

      it 'renders nothing' do
        result = render_inline(component)
        expect(result.to_html).to be_empty
      end
    end

    context 'when medication is in stock' do
      let(:current_supply) { 100 }

      it 'renders the supply count' do
        result = render_inline(component)
        expect(result.text).to include('100 left')
      end

      it 'does not allow the count to wrap in compact card headers' do
        result = render_inline(component)
        badge = result.at_css('span')

        expect(badge['class']).to include('whitespace-nowrap')
        expect(badge['class']).to include('shrink-0')
      end
    end

    context 'when medication is low stock' do
      let(:current_supply) { 5 }
      let(:status) { :low_stock }

      it 'renders Low Stock badge with count' do
        result = render_inline(component)
        expect(result.text).to include('Low Stock (5)')
      end
    end

    context 'when medication is out of stock' do
      let(:current_supply) { 0 }
      let(:status) { :out_of_stock }

      it 'renders Out of Stock badge with count' do
        result = render_inline(component)
        expect(result.text).to include('Out of Stock (0)')
      end
    end
  end
end
