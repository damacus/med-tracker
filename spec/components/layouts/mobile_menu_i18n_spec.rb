# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::MobileMenu, type: :component do
  describe 'i18n translations' do
    it 'renders mobile menu with default locale translations' do
      user = instance_double(User, administrator?: false, name: 'Test User')
      component = described_class.new(current_user: user)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('MedTracker')
      expect(rendered.to_html).to include('Open menu')
      expect(rendered.to_html).to include('Close menu')
    end
  end
end
