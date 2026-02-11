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
      rendered = render_inline(described_class.new(title: 'Medicines', value: 3, icon_type: 'pill'))

      expect(rendered.css('svg')).to be_present
    end
  end

  describe 'structure' do
    it 'renders within a card component' do
      rendered = render_inline(described_class.new(title: 'Test', value: 0, icon_type: 'users'))

      expect(rendered.css('[class*="h-full"]')).to be_present
    end

    it 'renders the title as a heading' do
      rendered = render_inline(described_class.new(title: 'Active Prescriptions', value: 10, icon_type: 'pill'))

      heading = rendered.css('h2')
      expect(heading).to be_present
      expect(heading.text).to include('Active Prescriptions')
    end
  end
end
