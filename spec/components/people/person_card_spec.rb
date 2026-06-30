# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::People::PersonCard, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:person) do
    build(:person, :dependent_adult, id: 1, name: 'Jane Doe')
  end

  it 'renders Needs Carer badge using RubyUI::Badge component (not inline styles)' do
    rendered = render_inline(described_class.new(person: person))

    badge = rendered.at_css('[data-testid="needs-carer-badge"]')
    expect(badge).to be_present, 'Expected Needs Carer badge to be rendered'
    expect(badge.name).to eq('span')
    expect(badge['class']).to include('border-warning/50'),
                              'Expected badge to use M3 badge tokens'
  end

  it 'prefixes the card id with the current household identity' do
    household = Household.create!(name: 'Component DOM Household', slug: 'component-dom-household')
    Current.household = household

    rendered = render_inline(described_class.new(person: person))

    expect(rendered.at_css("#household_#{household.id}_person_1")).to be_present
  ensure
    Current.reset
  end

  it 'checks carer relationship state once per render' do
    needs_carer = people(:one)
    needs_carer.association(:carers).reset

    expect(count_carer_relationship_queries { render_inline(described_class.new(person: needs_carer)) }).to eq(1)
  end

  it 'renders card actions with shared M3 sizing and shape' do
    Current.household = person.household

    component = described_class.new(person: person)
    allow(component).to receive(:can_create?).and_return(true)

    rendered = render_inline(component)
    action_links = rendered.css('a').select do |link|
      link.text.match?(/Add Medication|View Medications|Assign Carer/)
    end
    action_classes = action_links.map { |link| link[:class].split }

    expect(action_classes).not_to be_empty
    expect(action_classes).to all(include_touch_target_class)
    expect(action_classes).to all(include('rounded-shape-full'))
    expect(action_classes.flatten).not_to include('rounded-xl')
  ensure
    Current.reset
  end

  def include_touch_target_class
    satisfy { |classes| classes.include?('min-h-11') || classes.include?('min-h-[44px]') }
  end

  def count_carer_relationship_queries(&)
    count = 0

    subscriber = lambda do |_name, _start, _finish, _id, payload|
      sql = payload[:sql]
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      count += 1 if sql.include?('"carer_relationships"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
