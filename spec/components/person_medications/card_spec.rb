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

  it 'renders editable actions in a responsive action dock' do
    rendered = render_person_medication_card(update: true, destroy: true)

    expect(person_medication_action_dock_signature(rendered)).to eq(
      shell: ['@container'],
      dock: ['grid-cols-[minmax(0,1fr)_3rem]', '@[22rem]:grid-cols-[minmax(5.25rem,0.7fr)_minmax(0,1.4fr)_3rem]'],
      log: %w[order-1 col-span-2 @[22rem]:order-2],
      edit: %w[order-2 @[22rem]:order-1],
      delete: %w[order-3],
      actions: %i[log edit delete]
    )
  end

  def render_person_medication_card(update: false, destroy: false)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?, :show?).new(update, true, destroy, true)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(person_medication: person_medication, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def person_medication_action_dock_signature(rendered)
    shell = rendered.at_css('[data-testid="person-medication-action-shell"]')
    dock = rendered.at_css('[data-testid="person-medication-action-dock"]')
    log_action = dock.at_css('[data-testid="person-medication-log-action"]')
    edit_action = dock.at_css('[data-testid="person-medication-edit-action"]')
    delete_action = dock.at_css('[data-testid="person-medication-delete-action"]')

    {
      shell: class_tokens(shell).intersection(['@container']),
      dock: class_tokens(dock).intersection(
        ['grid-cols-[minmax(0,1fr)_3rem]', '@[22rem]:grid-cols-[minmax(5.25rem,0.7fr)_minmax(0,1.4fr)_3rem]']
      ),
      log: class_tokens(log_action).intersection(%w[order-1 col-span-2 @[22rem]:order-2]),
      edit: class_tokens(edit_action).intersection(%w[order-2 @[22rem]:order-1]),
      delete: class_tokens(delete_action).intersection(%w[order-3]),
      actions: person_medication_action_names(log_action, edit_action, delete_action)
    }
  end

  def person_medication_action_names(log_action, edit_action, delete_action)
    [
      (:log if log_action.at_css("button[data-testid='log-past-dose-person-medication-#{person_medication.id}']")),
      (:edit if edit_action.at_css("a[data-testid='edit-person-medication-#{person_medication.id}']")),
      (:delete if delete_action.at_css("button[data-testid='delete-person-medication-#{person_medication.id}']"))
    ].compact
  end

  def class_tokens(node)
    node['class'].split
  end
end
