# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Sidebar, type: :component do
  fixtures :accounts, :people, :users

  let(:admin_user) { users(:admin) }

  def render_sidebar(user: nil)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { user }
    # Mock current_page?
    allow(vc).to receive(:current_page?).and_return(false)

    html = vc.render(described_class.new(current_user: user))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  context 'when user is authenticated' do
    it 'renders the brand name' do
      rendered = render_sidebar(user: admin_user)
      expect(rendered.text).to include('MedTracker')
    end

    it 'renders navigation links' do
      rendered = render_sidebar(user: admin_user)
      expect(rendered.text).to include('Dashboard')
      expect(rendered.text).to include('Inventory')
      expect(rendered.text).to include('Reports')
    end

    it 'renders the user profile section' do
      rendered = render_sidebar(user: admin_user)
      expect(rendered.text).to include(admin_user.person.name)
      expect(rendered.text).to include('Administrator')
    end

    it 'renders the sign out button' do
      rendered = render_sidebar(user: admin_user)
      expect(rendered.text).to include('Sign Out')
    end
  end

  context 'when user is not authenticated' do
    it 'renders nothing' do
      rendered = render_sidebar(user: nil)
      expect(rendered.text.strip).to be_empty
    end
  end
end
