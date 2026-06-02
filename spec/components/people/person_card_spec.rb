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

  it 'checks carer relationship state once per render' do
    needs_carer = people(:one)
    needs_carer.association(:carers).reset

    expect(count_carer_relationship_queries { render_inline(described_class.new(person: needs_carer)) }).to eq(1)
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
