# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Navigation, type: :component do
  fixtures :accounts, :people, :users

  describe 'i18n translations' do
    it 'renders navigation with default locale translations' do
      component = described_class.new(current_user: nil)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('MedTracker')
      expect(rendered.to_html).to include('Skip to content')
      expect(rendered.to_html).to include('Login')
    end
  end

  describe 'brand readability' do
    it 'uses foreground text color utility for the brand link' do
      component = described_class.new(current_user: nil)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('nav__brand-link text-foreground')
    end
  end

  describe 'authenticated mobile actions' do
    it 'renders notification bell instead of the search trigger' do
      rendered = render_inline(described_class.new(current_user: users(:admin)))

      notifications_link = rendered.at_css("a[aria-label='Notifications']")

      expect(rendered.css('button[data-action="global-search#open"]')).to be_empty
      expect(rendered.css('.nav__right svg.lucide-search')).to be_empty
      expect(notifications_link).to be_present
      expect(notifications_link['href']).to eq('/profile#notifications-card')
      expect(notifications_link.at_css('svg.lucide-bell')).to be_present
    end
  end
end
