# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::QuickActions, type: :component do
  describe 'i18n translations' do
    it 'renders quick actions with default locale translations' do
      component = described_class.new

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Quick Actions')
      expect(rendered.to_html).to include('Add Medication')
    end
  end
end
