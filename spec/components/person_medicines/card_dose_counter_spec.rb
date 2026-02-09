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

  describe 'dose counter badge' do
    it 'meets WCAG 2.5.8 minimum target size of 24px' do
      rendered = render_card

      badge = rendered.css('span.rounded-full').first
      expect(badge).to be_present
      expect(badge['class']).to include('min-h-[24px]')
    end
  end
end
