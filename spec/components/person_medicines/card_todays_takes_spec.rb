# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::Card, type: :component do
  let(:person) { create(:person) }
  let(:medicine) { create(:medicine, name: 'Paracetamol') }
  let(:person_medicine) do
    PersonMedicine.create!(
      person: person,
      medicine: medicine,
      notes: 'Take with food',
      max_daily_doses: 4,
      min_hours_between_doses: 4
    )
  end

  def render_card(todays_takes: nil)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medicine?, :destroy?).new(false, false, false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    kwargs = { person_medicine: person_medicine, person: person }
    kwargs[:todays_takes] = todays_takes unless todays_takes.nil?
    html = vc.render(described_class.new(**kwargs))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  describe 'pre-loaded todays_takes' do
    let(:take) { MedicationTake.create!(person_medicine: person_medicine, taken_at: Time.current) }

    it 'displays takes from pre-loaded collection' do
      rendered = render_card(todays_takes: [take])
      expect(rendered.text).to include(take.taken_at.strftime('%l:%M %p').strip)
    end

    it 'shows no doses message when pre-loaded takes is empty' do
      rendered = render_card(todays_takes: [])
      expect(rendered.text).to include('No doses taken today')
    end

    it 'falls back to querying when todays_takes is not provided' do
      take
      rendered = render_card
      expect(rendered.text).to include(take.taken_at.strftime('%l:%M %p').strip)
    end
  end
end
