# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Prescriptions::Card, type: :component do
  let(:person) { create(:person) }
  let(:medicine) { create(:medicine, name: 'Ibuprofen') }
  let(:dosage) { Dosage.create!(medicine: medicine, amount: 400.0, unit: 'mg', frequency: 'Twice daily') }
  let(:prescription) do
    Prescription.create!(
      person: person,
      medicine: medicine,
      dosage: dosage,
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  def render_card(todays_takes: nil)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    kwargs = { prescription: prescription, person: person }
    kwargs[:todays_takes] = todays_takes unless todays_takes.nil?
    html = vc.render(described_class.new(**kwargs))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  describe 'pre-loaded todays_takes' do
    let(:take) { MedicationTake.create!(prescription: prescription, taken_at: Time.current, amount_ml: 400) }

    it 'displays takes from pre-loaded collection' do
      rendered = render_card(todays_takes: [take])
      expect(rendered.text).to include('400 mg')
    end

    it 'shows no doses message when pre-loaded takes is empty' do
      rendered = render_card(todays_takes: [])
      expect(rendered.text).to include('No doses taken today')
    end

    it 'falls back to querying when todays_takes is not provided' do
      take
      rendered = render_card
      expect(rendered.text).to include('400 mg')
    end
  end
end
