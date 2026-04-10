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
end
