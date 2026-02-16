# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::StockBadge, type: :component do
  subject(:component) { described_class.new(medicine: medicine) }

  let(:medicine) { instance_double(Medicine, stock: stock, low_stock?: low_stock, out_of_stock?: out_of_stock) }
  let(:stock) { 100 }
  let(:low_stock) { false }
  let(:out_of_stock) { false }

  describe '#view_template' do
    context 'when medicine has no stock value' do
      let(:stock) { nil }

      it 'renders nothing' do
        allow(medicine).to receive(:stock).and_return(nil)
        result = render_inline(component)
        expect(result.to_html).to be_empty
      end
    end

    context 'when medicine is in stock' do
      let(:stock) { 100 }

      it 'renders nothing (badge only shows for low/out of stock)' do
        result = render_inline(component)
        expect(result.to_html).to be_empty
      end
    end

    context 'when medicine is low stock' do
      let(:stock) { 5 }
      let(:low_stock) { true }

      it 'renders Low Stock badge' do
        result = render_inline(component)
        expect(result.text).to include('Low Stock')
      end
    end

    context 'when medicine is out of stock' do
      let(:stock) { 0 }
      let(:out_of_stock) { true }

      it 'renders Out of Stock badge' do
        result = render_inline(component)
        expect(result.text).to include('Out of Stock')
      end
    end
  end
end
