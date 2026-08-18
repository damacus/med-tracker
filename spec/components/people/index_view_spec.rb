# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::People::IndexView, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  it 'renders the new person action for managers' do
    rendered = render_people_index

    expect(rendered.at_css("a[href='#{view_context.new_person_path}']")).to be_present
  end

  it 'hides the new person action when creation is not allowed' do
    rendered = render_people_index(create_allowed: false)

    expect(rendered.at_css("a[href='#{view_context.new_person_path}']")).to be_nil
  end

  def render_people_index(create_allowed: true)
    vc = view_context
    policy_stub = Struct.new(:new?, :show?, :create?).new(create_allowed, true, true)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(people: [people(:john)]))

    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
