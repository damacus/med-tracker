# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::FormView, type: :component do
  describe 'i18n translations' do
    it 'renders form with default locale translations' do
      medicine = Medicine.new(name: 'Test Medicine')
      component = described_class.new(
        medicine: medicine,
        title: 'Test Title',
        subtitle: 'Test Subtitle'
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Name')
      expect(rendered.to_html).to include('Description')
      expect(rendered.to_html).to include('Save Medicine')
    end
  end
end
