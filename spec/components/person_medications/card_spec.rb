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

  it 'keeps secondary actions in an actions menu', :aggregate_failures do
    rendered = render_person_medication_card(update: true)

    trigger = rendered.at_css("button[data-testid='person-medication-actions-#{person_medication.id}']")
    trigger_classes = trigger['class'].split
    link = rendered.at_css("a[data-testid='edit-person-medication-#{person_medication.id}']")
    pause_button = rendered.at_css("button[data-testid='pause-person-medication-#{person_medication.id}']")

    expect(trigger).not_to be_nil
    expect(trigger_classes).to include('h-11')
    expect(trigger_classes).to include('w-11')
    expect(trigger.at_css('.sr-only').text).to eq('Actions')
    expect(link).not_to be_nil
    expect(link['role']).to eq('menuitem')
    expect(link.text).to include('Edit')
    expect(pause_button['role']).to eq('menuitem')
    expect(pause_button.text).to include('Pause medication')
  end

  it 'renders pause action for manageable active assignments' do
    rendered = render_person_medication_card(update: true)

    expect(rendered.at_css("button[data-testid='pause-person-medication-#{person_medication.id}']")).to be_present
  end

  it 'keeps the card footer on one line without clipping secondary actions' do
    rendered = render_person_medication_card(update: true, destroy: true)
    actions = rendered.at_css('[data-testid="person-medication-card-actions"]')
    action_classes = actions['class'].split
    menu = rendered.at_css("[data-testid='person-medication-actions-menu-#{person_medication.id}']")

    expect(action_classes).not_to include('flex-wrap')
    expect(action_classes).to include('min-w-0')
    expect(menu).not_to be_nil
    expect(rendered.at_css("button[data-testid='delete-person-medication-#{person_medication.id}']")).to be_present
  end

  it 'uses design-system sizing for compact action controls', :aggregate_failures do
    rendered = render_person_medication_card(update: true, destroy: true)
    past_dose_button_classes = rendered.at_css(
      "button[data-testid='log-past-dose-person-medication-#{person_medication.id}']"
    )['class'].split
    dropdown = rendered.at_css('[data-ruby-ui--dropdown-menu-options-value]')

    expect(past_dose_button_classes).to include('min-h-[44px]')
    expect(past_dose_button_classes).not_to include('h-14')
    expect(past_dose_button_classes).not_to include('min-h-[56px]')
    expect(past_dose_button_classes).not_to include('py-6')
    expect(dropdown['data-ruby-ui--dropdown-menu-options-value']).to include('"strategy":"fixed"')
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
