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
      dock: ['grid-cols-[minmax(0,1fr)_3rem]', 'gap-y-4', 'bg-surface-container-high', 'p-4'],
      log: %w[col-span-2],
      edit: %w[min-w-0],
      delete: %w[order-3],
      log_button: %w[h-14 rounded-full shadow-elevation-2],
      edit_button: %w[h-14 rounded-3xl bg-surface-container-lowest],
      delete_button: %w[bg-transparent text-error],
      reorder: %w[rounded-3xl bg-surface-container-low py-4],
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
    nodes = person_medication_action_nodes(rendered)

    person_medication_action_layout_signature(nodes).merge(
      person_medication_button_signature(nodes, rendered),
      actions: person_medication_action_names(nodes)
    )
  end

  def person_medication_action_nodes(rendered)
    dock = rendered.at_css('[data-testid="person-medication-action-dock"]')
    {
      dock: dock,
      log: dock.at_css('[data-testid="person-medication-log-action"]'),
      edit: dock.at_css('[data-testid="person-medication-edit-action"]'),
      delete: dock.at_css('[data-testid="person-medication-delete-action"]'),
      log_button: dock.at_css("button[data-testid='log-past-dose-person-medication-#{person_medication.id}']"),
      edit_button: dock.at_css("a[data-testid='edit-person-medication-#{person_medication.id}']"),
      delete_button: dock.at_css("button[data-testid='delete-person-medication-#{person_medication.id}']")
    }
  end

  def person_medication_action_layout_signature(nodes)
    {
      dock: class_tokens(nodes[:dock]).intersection(
        ['grid-cols-[minmax(0,1fr)_3rem]', 'gap-y-4', 'bg-surface-container-high', 'p-4']
      ),
      log: class_tokens(nodes[:log]).intersection(%w[col-span-2]),
      edit: class_tokens(nodes[:edit]).intersection(%w[min-w-0]),
      delete: class_tokens(nodes[:delete]).intersection(%w[order-3])
    }
  end

  def person_medication_button_signature(nodes, rendered)
    {
      log_button: class_tokens(nodes[:log_button]).intersection(%w[h-14 rounded-full shadow-elevation-2]),
      edit_button: class_tokens(nodes[:edit_button]).intersection(%w[h-14 rounded-3xl bg-surface-container-lowest]),
      delete_button: class_tokens(nodes[:delete_button]).intersection(%w[text-error bg-transparent]),
      reorder: class_tokens(reorder_controls(rendered)).intersection(%w[rounded-3xl bg-surface-container-low py-4])
    }
  end

  def reorder_controls(rendered)
    rendered.at_css("button[data-testid='move-up-person-medication-#{person_medication.id}']").parent.parent
  end

  def person_medication_action_names(nodes)
    [
      (:log if nodes[:log_button]),
      (:edit if nodes[:edit_button]),
      (:delete if nodes[:delete_button])
    ].compact
  end

  def class_tokens(node)
    node['class'].split
  end
end
