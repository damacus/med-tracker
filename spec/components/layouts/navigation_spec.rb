# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Navigation, type: :component do
  fixtures :users

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

  describe 'accessibility' do
    it 'hides the search icon from the labelled search button' do
      rendered = render_inline(described_class.new(current_user: users(:admin)))

      search_button = rendered.at_css(%(button[aria-label="#{I18n.t('global_search.open')}"]))

      expect(search_button).to be_present
      expect(search_button.at_css('svg[aria-hidden="true"]')).to be_present
    end
  end
end
