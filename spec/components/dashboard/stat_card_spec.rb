# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::StatCard, type: :component do
  describe 'rendering' do
    it 'renders the title' do
      rendered = render_inline(described_class.new(title: 'People', value: 5, icon_type: 'users'))

      expect(rendered.text).to include('People')
    end

    it 'renders the value' do
      rendered = render_inline(described_class.new(title: 'People', value: 42, icon_type: 'users'))

      expect(rendered.text).to include('42')
    end

    it 'renders a users icon when icon_type is users' do
      rendered = render_inline(described_class.new(title: 'People', value: 5, icon_type: 'users'))

      expect(rendered.css('svg')).to be_present
    end

    it 'renders a pill icon when icon_type is pill' do
      rendered = render_inline(described_class.new(title: 'Medications', value: 3, icon_type: 'pill'))

      expect(rendered.css('svg')).to be_present
    end
  end

  describe 'structure' do
    it 'renders within a card component' do
      rendered = render_inline(described_class.new(title: 'Test', value: 0, icon_type: 'users'))

      expect(rendered.css('[class*="bg-surface-container-low"]')).to be_present
    end

    it 'renders the title' do
      rendered = render_inline(described_class.new(title: 'Active Schedules', value: 10, icon_type: 'pill'))

      expect(rendered.text).to include('Active Schedules')
    end

    it 'renders a link wrapper when href is provided' do
      rendered = render_inline(described_class.new(title: 'People', value: 5, icon_type: 'users', href: '/people'))

      expect(rendered.css('a[href="/people"]')).to be_present
      expect(rendered.to_html).not_to include('h-9')
    end

    it 'does not render a link wrapper when href is omitted' do
      rendered = render_inline(described_class.new(title: 'People', value: 5, icon_type: 'users'))

      expect(rendered.css('a')).to be_empty
    end
  end
end
