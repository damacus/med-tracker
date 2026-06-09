# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::People::IndexView, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  it 'renders a row per person, linking to the person record' do
    person = people(:john)
    rendered = render_inline(described_class.new(people: [person]))

    expect(rendered.text).to include(person.name)
    expect(rendered.at_css("a[href='/people/#{person.id}']")).to be_present
  end

  it 'renders a quiet empty state when no people need carers' do
    rendered = render_inline(described_class.new(people: []))

    expect(rendered.at_css('[data-testid="admin-people-empty"]')).to be_present
    expect(rendered.text).to include(I18n.t('admin.people.index.empty'))
  end
end
