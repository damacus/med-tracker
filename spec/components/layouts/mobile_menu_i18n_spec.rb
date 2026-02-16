# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::MobileMenu, type: :component do
  describe 'i18n translations' do
    it 'renders mobile menu with translated brand' do
      user = instance_double(User, administrator?: false, name: 'Test User')
      component = described_class.new(current_user: user)

      allow(component).to receive(:t).with('layouts.mobile_menu.brand').and_return('MedTracker')

      render_inline(component)

      expect(rendered_content).to include('MedTracker')
    end
  end
end
