# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Navigation, type: :component do
  describe 'i18n translations' do
    it 'renders with translated brand name' do
      component = described_class.new(current_user: nil)

      allow(component).to receive(:t).with('layouts.navigation.brand').and_return('MedTracker')
      allow(component).to receive(:t).with('layouts.navigation.skip_to_content').and_return('Skip to content')
      allow(component).to receive(:t).with('layouts.navigation.login').and_return('Login').twice

      render_inline(component)

      expect(rendered_content).to include('MedTracker')
    end
  end
end
