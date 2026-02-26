# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen') }

  let(:schedule) do
    dosage = Dosage.create!(medication: medication, amount: 400.0, unit: 'mg', frequency: 'Twice daily')
    Schedule.create!(
      person: person,
      medication: medication,
      dosage: dosage,
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  it 'displays dosage unit from schedule, not hardcoded ml' do
    MedicationTake.create!(schedule: schedule, taken_at: Time.current, amount_ml: 400)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }

    html = vc.render(described_class.new(schedule: schedule, person: person))
    rendered = Nokogiri::HTML::DocumentFragment.parse(html)

    take_text = rendered.text
    expect(take_text).to include('400mg')
    expect(take_text).not_to match(/400\s*ml/i)
  end
end
