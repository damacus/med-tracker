# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::Card, type: :component do
  let(:person) { create(:person) }
  let(:medicine) { create(:medicine, name: 'Paracetamol') }

  let(:person_medicine) do
    PersonMedicine.create!(
      person: person,
      medicine: medicine,
      max_daily_doses: 4,
      min_hours_between_doses: 4
    )
  end

  def render_card
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:take_medicine?, :destroy?).new(false, false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }

    html = vc.render(described_class.new(person_medicine: person_medicine, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  describe 'medicine icon color' do
    it 'uses violet styling consistent with Prescriptions::Card' do
      rendered = render_card

      icon_container = rendered.css('div').detect { |d| d['class']&.include?('w-10') && d['class'].include?('h-10') }
      expect(icon_container).to be_present, 'Expected to find a 10x10 icon container div'
      expect(icon_container['class']).to include('bg-violet-100')
      expect(icon_container['class']).to include('text-violet-700')
    end
  end
end
