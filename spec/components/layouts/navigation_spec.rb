# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Navigation, type: :component do
  describe 'i18n translations' do
    it 'renders navigation with default locale translations' do
      component = described_class.new(current_user: nil)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('MedTracker')
      expect(rendered.to_html).to include('Skip to content')
      expect(rendered.to_html).to include('Login')
    end
  end
end
