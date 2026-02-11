# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::ProfileMenu, type: :component do
  fixtures :accounts, :people, :users

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }

  describe 'rendering' do
    it 'renders a dropdown menu' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include(admin_user.name)
    end

    it 'renders My Account label' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include('My Account')
    end
  end

  describe 'menu items' do
    it 'renders Dashboard link' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include('Dashboard')
    end

    it 'renders Profile link' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include('Profile')
    end

    it 'renders Logout link' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include('Logout')
    end
  end

  describe 'admin menu item' do
    it 'renders Administration link for admin users' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include('Administration')
    end

    it 'does not render Administration link for non-admin users' do
      rendered = render_inline(described_class.new(current_user: carer_user))

      expect(rendered.text).not_to include('Administration')
    end
  end

  describe 'without current_user' do
    it 'renders Account as fallback name' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('Account')
    end
  end
end
