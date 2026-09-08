# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, dose_amount: 1, dose_unit: 'tablet') }
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

  it 'renders the actions menu trigger as a RubyUI outline button', :aggregate_failures do
    rendered = render_person_medication_card(update: true)

    trigger = rendered.at_css("button[data-testid='person-medication-actions-#{person_medication.id}']")
    trigger_classes = trigger['class'].split

    expect(trigger).not_to be_nil
    expect(trigger_classes).to include('bg-background')
    expect(trigger_classes).to include('min-h-[44px]')
    expect(trigger_classes).to include('px-4')
    expect(trigger_classes).not_to include('px-1.5')
    expect(trigger_classes).not_to include('state-layer')
    expect(trigger.text).to include('Actions')
  end

  it 'keeps secondary actions in an actions menu', :aggregate_failures do
    rendered = render_person_medication_card(update: true)

    link = rendered.at_css("a[data-testid='edit-person-medication-#{person_medication.id}']")
    pause_button = rendered.at_css("button[data-testid='pause-person-medication-#{person_medication.id}']")

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

  it 'stacks card actions on narrow screens and preserves secondary actions' do
    rendered = render_person_medication_card(update: true, destroy: true)
    actions = rendered.at_css('[data-testid="person-medication-card-actions"]')
    action_classes = actions['class'].split
    menu = rendered.at_css("[data-testid='person-medication-actions-menu-#{person_medication.id}']")

    expect(action_classes).to include('flex-col', 'lg:flex-row', 'items-stretch', 'w-full')
    expect(menu).not_to be_nil
    expect(rendered.at_css("button[data-testid='delete-person-medication-#{person_medication.id}']")).to be_present
  end

  it 'keeps the delete dialog trigger full-width inside the actions menu', :aggregate_failures do
    rendered = render_person_medication_card(update: true, destroy: true)

    expect(delete_dialog_classes(rendered)).to include('block', 'w-full')
    expect(delete_trigger_classes(rendered)).to include('block', 'w-full')
  end

  it 'uses design-system sizing for compact action controls', :aggregate_failures do
    rendered = render_person_medication_card(update: true, destroy: true)
    past_dose_button_classes = rendered.at_css(
      "button[data-testid='log-past-dose-person-medication-#{person_medication.id}']"
    )['class'].split
    dropdown = rendered.at_css('[data-ruby-ui--dropdown-menu-options-value]')

    expect(past_dose_button_classes).to include('min-h-[44px]')
    expect(past_dose_button_classes).to include('state-layer-overflow-visible')
    expect(past_dose_button_classes).not_to include('overflow-hidden')
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

  it 'extracts focused subcomponents for the card regions' do
    expect(described_class::HeaderComponent).to be < Components::Base
    expect(described_class::ContentComponent).to be < Components::Base
    expect(described_class::TimingStatusComponent).to be < Components::Base
    expect(described_class::ActionsComponent).to be < Components::Base
  end

  it 'renders the header component independently' do
    rendered = render_component(described_class::HeaderComponent.new(person_medication: person_medication))

    expect(rendered.at_css('h3').text).to include(medication.display_name)
  end

  it 'renders the content component independently' do
    person_medication.update!(notes: 'Take with food')
    rendered = render_component(described_class::ContentComponent.new(person_medication: person_medication))

    expect(rendered.text).to include('Take with food')
  end

  it 'renders the timing status component independently' do
    {
      ['daily', 1, 'Maximum 1 dose per day'] => 'Wait at least 1 hour between doses',
      ['weekly', 4, 'Maximum 4 doses per week'] => 'Wait at least 4 hours between doses',
      ['monthly', 2, 'Maximum 2 doses per month'] => 'Wait at least 4 hours between doses'
    }.each do |(cycle, max_doses, max_doses_copy), wait_hours_copy|
      person_medication.update!(dose_cycle: cycle, max_daily_doses: max_doses,
                                min_hours_between_doses: wait_hours_copy.start_with?('Wait at least 1') ? 1 : 4)
      rendered = render_component(described_class::TimingStatusComponent.new(person_medication: person_medication))

      expect(rendered.text).to include(max_doses_copy, wait_hours_copy)
      expect(rendered.text).not_to include('dose(s)')
    end
  end

  it 'uses translated singular and plural timing copy for every supported locale' do
    %i[en cy es ga pt].each do |locale|
      I18n.with_locale(locale) do
        person_medication.update!(dose_cycle: :weekly, max_daily_doses: 1, min_hours_between_doses: 1)
        singular = render_component(described_class::TimingStatusComponent.new(person_medication: person_medication))

        expect(singular.text).to include(I18n.t('person_medications.card.max_doses.weekly', count: 1))
        expect(singular.text).to include(I18n.t('person_medications.card.wait_hours', count: 1))

        person_medication.update!(max_daily_doses: 4, min_hours_between_doses: 4)
        plural = render_component(described_class::TimingStatusComponent.new(person_medication: person_medication))

        expect(plural.text).to include(I18n.t('person_medications.card.max_doses.weekly', count: 4))
        expect(plural.text).to include(I18n.t('person_medications.card.wait_hours', count: 4))
      end
    end
  end

  it 'normalises a missing or invalid cycle to the daily display period' do
    person_medication.update!(max_daily_doses: 1)
    allow(person_medication).to receive(:dose_cycle).and_return(nil)
    rendered = render_component(described_class::TimingStatusComponent.new(person_medication: person_medication))

    expect(rendered.text).to include(I18n.t('person_medications.card.max_doses.daily', count: 1))
    expect(rendered.text).not_to include('per week', 'per month')

    allow(person_medication).to receive(:dose_cycle).and_return('fortnightly')
    rendered = render_component(described_class::TimingStatusComponent.new(person_medication: person_medication))

    expect(rendered.text).to include(I18n.t('person_medications.card.max_doses.daily', count: 1))
  end

  it 'renders the actions component independently' do
    rendered = render_component(
      described_class::ActionsComponent.new(person_medication: person_medication, person: person, current_user: nil),
      update: true
    )

    expect(rendered.at_css('[data-testid="person-medication-card-actions"]')).to be_present
  end

  def render_person_medication_card(update: false, destroy: false)
    render_component(described_class.new(person_medication: person_medication, person: person), update:, destroy:)
  end

  def render_component(component, update: false, destroy: false)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?, :show?).new(update, true, destroy, true)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(component)
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def delete_trigger_classes(rendered)
    button = rendered.at_css("button[data-testid='delete-person-medication-#{person_medication.id}']")
    button.ancestors.find { |node| node['data-action'] == 'click->ruby-ui--alert-dialog#open' }['class'].split
  end

  def delete_dialog_classes(rendered)
    button = rendered.at_css("button[data-testid='delete-person-medication-#{person_medication.id}']")
    button.ancestors.find { |node| node['data-controller'] == 'ruby-ui--alert-dialog' }['class'].split
  end
end
