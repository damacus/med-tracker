# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Prescriptions::Card, type: :component do
  let(:person) { create(:person) }
  let(:medicine) { create(:medicine, name: 'Ibuprofen') }

  let(:prescription) do
    dosage = Dosage.create!(medicine: medicine, amount: 400.0, unit: 'mg', frequency: 'Twice daily')
    Prescription.create!(
      person: person,
      medicine: medicine,
      dosage: dosage,
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  it 'displays dosage unit from prescription, not hardcoded ml' do
    MedicationTake.create!(prescription: prescription, taken_at: Time.current, amount_ml: 400)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }

    html = vc.render(described_class.new(prescription: prescription, person: person))
    rendered = Nokogiri::HTML::DocumentFragment.parse(html)

    take_text = rendered.text
    expect(take_text).to include('400mg')
    expect(take_text).not_to match(/400\s*ml/i)
  end
end
