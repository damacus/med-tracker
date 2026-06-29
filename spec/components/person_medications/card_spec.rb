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

  it 'keeps the edit action from shrinking in the card footer' do
    rendered = render_person_medication_card(update: true)

    link = rendered.at_css("a[data-testid='edit-person-medication-#{person_medication.id}']")

    expect(link).not_to be_nil
    expect(link['class']).to include('shrink-0')
    expect(link['class']).to include('min-w-12')
  end

  it 'renders pause action for manageable active assignments' do
    rendered = render_person_medication_card(update: true)

    expect(rendered.at_css("button[data-testid='pause-person-medication-#{person_medication.id}']")).to be_present
  end

  it 'renders paused state without dose actions' do
    person_medication.update!(active: false)

    rendered = render_person_medication_card(update: true)

    expect(rendered.text).to include('Paused')
    expect(rendered.at_css("button[data-testid='log-past-dose-person-medication-#{person_medication.id}']")).to be_nil
    expect(rendered.at_css("button[data-testid='resume-person-medication-#{person_medication.id}']")).to be_present
  end

  def render_person_medication_card(update: false, destroy: false)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?, :show?).new(update, true, destroy, true)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(person_medication: person_medication, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
