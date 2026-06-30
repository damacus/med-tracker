# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::IndexView, type: :component do
  fixtures :accounts, :people, :users

  def render_view(policy_stub: nil, **args)
    view_context_helper = view_context
    policy_stub ||= Struct.new(:create?, :update?, :refill?, :destroy?).new(true, true, true, true)
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

  it 'renders scan stock for users who can restock but cannot update medication details' do
    rendered = render_view(
      medications: [],
      policy_stub: Struct.new(:create?, :update?, :refill?, :destroy?).new(false, false, true, false)
    )

    expect(rendered.text).to include('Scan stock')
    expect(rendered.text).not_to include('Add Medication')
  end

  it 'renders restock but not edit or delete actions for restock-only medication access' do
    medication = create(:medication)
    rendered = render_view(
      medications: [medication],
      policy_stub: Struct.new(:create?, :update?, :refill?, :destroy?).new(false, false, true, false)
    )

    expect(rendered.css("button[aria-label='Restock']")).to be_present
    edit_path = Rails.application.routes.url_helpers.edit_medication_path(
      household_slug: 'test-household',
      id: medication
    )

    expect(rendered.css("a[href^='#{edit_path}']")).to be_empty
    expect(rendered.text).not_to include('Delete medication')
  end

  it 'wraps header actions on mobile while preserving desktop inline layout' do
    rendered = render_view(medications: [])

    actions = rendered.at_css('.medications-index-actions')

    expect(actions[:class]).to include('flex-wrap')
    expect(actions[:class]).to include('md:flex-nowrap')
    expect(actions[:class]).to include('w-full')
    expect(actions[:class]).to include('md:w-auto')
  end

  it 'keeps header action controls compact while allowing wrapping' do
    rendered = render_view(medications: [])
    actions = rendered.at_css('.medications-index-actions')

    scan_button = actions.at_css('button')
    add_schedule_link = actions.css('a').find { |link| link.text.include?('Add Schedule') }
    add_medication_link = actions.css('a').find { |link| link.text.include?('Add Medication') }
    action_classes = [scan_button, add_schedule_link, add_medication_link].map { |element| element[:class].to_s }

    expect(action_classes).to all(include('max-w-full'))
    expect(action_classes.flat_map(&:split)).not_to include('w-full')
  end
end
