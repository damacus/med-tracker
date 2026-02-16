# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::QuickActions, type: :component do
  describe 'i18n translations' do
    it 'renders quick actions with translated title' do
      url_helpers = instance_double(UrlHelpers)
      component = described_class.new(url_helpers: url_helpers)

      allow(component).to receive(:t).with('dashboard.quick_actions.title').and_return('Quick Actions')

      render_inline(component)

      expect(rendered_content).to include('Quick Actions')
    end
  end
end
