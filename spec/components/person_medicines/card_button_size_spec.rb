# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::Card, type: :component do
  let(:person) { create(:person) }
  let(:medicine) { create(:medicine, name: 'Paracetamol') }
  let(:user) { create(:user, :administrator) }

  let(:person_medicine) do
    PersonMedicine.create!(
      person: person,
      medicine: medicine,
      max_daily_doses: 4,
      min_hours_between_doses: 4
    )
  end

  def render_card_with_user
    vc = view_context
    vc.singleton_class.define_method(:current_user) { user }
    policy_stub = Struct.new(:take_medicine?, :destroy?).new(true, false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }

    html = vc.render(described_class.new(person_medicine: person_medicine, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  describe 'take button size' do
    it 'uses :md size matching Prescriptions::Card for consistency' do
      rendered = render_card_with_user

      take_button = rendered.at_css('button[type="submit"]')
      expect(take_button).to be_present

      button_classes = take_button['class']
      expect(button_classes).to include('h-9'), 'Take button should use :md size (h-9) not :sm (h-8)'
    end
  end
end
