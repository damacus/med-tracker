# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::IndexView, type: :component do
  fixtures :accounts, :people, :users

  def render_view(**args)
    view_context_helper = view_context
    policy_stub = Struct.new(:create?, :update?).new(true, true)
    admin = users(:admin)

    view_context_helper.singleton_class.define_method(:policy) { |_record| policy_stub }
    view_context_helper.singleton_class.define_method(:current_user) { admin }
    view_context_helper.singleton_class.define_method(:pundit_user) { admin }

    html = view_context_helper.render(described_class.new(**args))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  it 'renders the empty state when there are no medications' do
    rendered = render_view(medications: [])

    expect(rendered.text).to include('No medications yet')
    expect(rendered.text).to include('Add Medication')
  end

  it 'offsets the content to align with the header title column' do
    rendered = render_view(medications: [])

    content = rendered.at_css("[data-testid='medications-content']")

    expect(content).to be_present
    expect(content[:class]).to include('md:pl-[6.5rem]')
  end

  it 'renders medication actions with m3 link variants' do
    rendered = render_view(medications: [])

    add_schedule_link = rendered.css('a').find { |link| link.text.include?('Add Schedule') }
    add_medication_link = rendered.css('a').find { |link| link.text.include?('Add Medication') }

    expect(add_schedule_link[:class]).to include('rounded-full')
    expect(add_schedule_link[:class]).to include('hover:bg-tertiary-container')
    expect(add_schedule_link[:class]).to include('bg-card')
    expect(add_medication_link[:class]).to include('bg-primary')
    expect(add_medication_link[:class]).to include('text-primary-foreground')
  end
end
