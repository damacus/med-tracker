# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::IndexView, type: :component do
  let(:medication) { create(:medication, name: 'Paracetamol', current_supply: 16, dosage_unit: 'sachet') }
  let(:person) { create(:person, name: 'John Doe') }

  def render_view(medications:)
    vc = view_context
    policy_stub = Struct.new(:create?, :update?, :destroy?).new(true, true, true)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }

    html = vc.render(described_class.new(medications: medications))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  it 'renders unit-aware supply label' do
    page = render_view(medications: [medication])
    expect(page.text).to include('16 sachets')
  end

  it 'renders singular supply label' do
    medication.update!(current_supply: 1)
    page = render_view(medications: [medication])
    expect(page.text).to include('1 sachet')
  end

  it 'renders assignee badges' do
    create(:person_medication, medication: medication, person: person)
    page = render_view(medications: [medication])
    expect(page.text).to include('John')
  end

  it 'renders multiple assignee badges' do
    person2 = create(:person, name: 'Jane Smith')
    create(:person_medication, medication: medication, person: person)
    create(:person_medication, medication: medication, person: person2)

    page = render_view(medications: [medication])
    expect(page.text).to include('John')
    expect(page.text).to include('Jane')
  end
end
