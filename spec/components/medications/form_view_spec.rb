# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::FormView, type: :component do
  describe 'i18n translations' do
    it 'renders form with default locale translations' do
      medication = Medication.new(name: 'Test Medication')
      component = described_class.new(
        medication: medication,
        title: 'Test Title',
        subtitle: 'Test Subtitle'
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Name')
      expect(rendered.to_html).to include('Description')
      expect(rendered.to_html).to include('Save Medication')
    end
  end
end
