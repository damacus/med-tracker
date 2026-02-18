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
end
