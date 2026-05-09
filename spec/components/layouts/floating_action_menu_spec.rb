# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::FloatingActionMenu, type: :component do
  fixtures :accounts, :people, :users

  let(:carer_user) { users(:carer) }
  let(:admin_user) { users(:admin) }

  def render_menu(user:, path:)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { user }
    allow(vc.request).to receive(:path).and_return(path)

    html = vc.render(described_class.new(current_user: user))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  it 'renders quick actions on allowed pages' do
    rendered = render_menu(user: carer_user, path: Rails.application.routes.url_helpers.root_path)

    expect(rendered.css('button[aria-label="Open quick actions"]')).to be_present
    expect(rendered.text).to include('Add Medication', 'Add Person')
  end

  it 'hides the menu on disallowed pages' do
    rendered = render_menu(user: admin_user, path: Rails.application.routes.url_helpers.profile_path)

    expect(rendered.text.strip).to be_empty
  end
end
