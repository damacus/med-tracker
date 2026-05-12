# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, dosage_amount: 1, dosage_unit: 'tablet') }
  let(:person_medication) do
    create(:person_medication, person: person, medication: medication, max_daily_doses: nil,
                               min_hours_between_doses: nil)
  end

  it 'wraps full medication names inside the card header' do
    medication.update!(name: 'Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)')
    rendered = render_person_medication_card

    title = rendered.at_css('h3')
    expect(title.text).to include(medication.name)
    expect(title['class']).to include('break-words')
  end

  it 'renders the friendly medication display name when present' do
    medication.update!(
      name: 'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets',
      friendly_name: 'Movicol Paediatric Plain'
    )
    rendered = render_person_medication_card

    title = rendered.at_css('h3')
    expect(title.text).to include('Movicol Paediatric Plain')
    expect(title.text).not_to include('Norgine Pharmaceuticals')
  end

  it 'renders a Log a past dose button' do
    rendered = render_person_medication_card

    button = rendered.at_css("button[data-testid='log-past-dose-person-medication-#{person_medication.id}']")

    expect(button).not_to be_nil
    expect(button.text).to include('Log a past dose')
  end

  def render_person_medication_card
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?, :show?).new(false, true, false, true)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(person_medication: person_medication, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
