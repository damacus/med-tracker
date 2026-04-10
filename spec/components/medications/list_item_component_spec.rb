# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::ListItemComponent, type: :component do
  let(:medication) do
    create(:medication,
           name: 'Paracetamol',
           description: 'Pain relief',
           current_supply: 50,
           reorder_threshold: 10)
  end

  it 'renders the medication details and primary actions' do
    rendered = render_inline(described_class.new(
                               medication: medication,
                               inventory_query_params: { category: 'Vitamin' },
                               can_manage: true
                             ))

    expect(rendered.text).to include('Paracetamol')
    expect(rendered.text).to include('Pain relief')
    expect(rendered.text).to include('Inventory Level')
    expect(rendered.text).to include('50 units')
    expect(rendered.css("a[href='/medications/#{medication.id}']")).to be_present
  end

  it 'omits the refill action when the medication cannot be managed' do
    rendered = render_inline(described_class.new(
                               medication: medication,
                               inventory_query_params: {},
                               can_manage: false
                             ))

    expect(rendered.text).not_to include('Refill Inventory')
  end
end
