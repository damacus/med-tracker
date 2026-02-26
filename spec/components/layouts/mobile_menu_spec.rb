# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::MobileMenu, type: :component do
  fixtures :accounts, :people, :users

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }

  describe 'rendering' do
    it 'renders within a mobile-only container' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('div.md\\:hidden')).to be_present
    end

    it 'renders a hamburger menu trigger' do
      rendered = render_inline(described_class.new)

      trigger = rendered.css('button[aria-label="Open menu"]')
      expect(trigger).to be_present
    end
  end

  describe 'navigation links' do
    it 'renders Inventory link' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('Inventory')
    end

    it 'renders People link' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('People')
    end

    it 'renders Medication Finder link' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('Medication Finder')
    end
  end

  describe 'auth actions' do
    it 'renders Profile link' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('Profile')
    end

    it 'renders Logout button' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('Logout')
    end

    it 'renders Administration link for admin users' do
      rendered = render_inline(described_class.new(current_user: admin_user))

      expect(rendered.text).to include('Administration')
    end

    it 'does not render Administration link for non-admin users' do
      rendered = render_inline(described_class.new(current_user: carer_user))

      expect(rendered.text).not_to include('Administration')
    end
  end

  describe 'accessibility' do
    it 'renders hamburger button with aria-label' do
      rendered = render_inline(described_class.new)

      button = rendered.css('button[aria-label="Open menu"]')
      expect(button).to be_present
    end

    it 'renders close button with aria-label' do
      rendered = render_inline(described_class.new)

      close_button = rendered.css('button[aria-label="Close menu"]')
      expect(close_button).to be_present
    end

    it 'renders close button with 44px minimum touch target' do
      rendered = render_inline(described_class.new)

      close_button = rendered.css('button[aria-label="Close menu"]').first
      expect(close_button['class']).to include('min-h-[44px]')
      expect(close_button['class']).to include('min-w-[44px]')
    end

    it 'renders sr-only text for close button' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('.sr-only').map(&:text)).to include('Close menu')
    end
  end

  describe 'keyboard navigation' do
    it 'renders close button with focus ring styles' do
      rendered = render_inline(described_class.new)

      close_button = rendered.css('button[aria-label="Close menu"]').first
      expect(close_button['class']).to include('focus:ring-2')
    end
  end
end
