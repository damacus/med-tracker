# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::FormView, type: :component do
  describe 'i18n translations' do
    it 'renders form with translated name field' do
      medicine = Medicine.new(name: 'Test Medicine')
      component = described_class.new(
        medicine: medicine,
        title: 'Test Title',
        subtitle: 'Test Subtitle'
      )

      allow(component).to receive(:t).with('forms.medicines.name').and_return('Name')

      render_inline(component)

      expect(rendered_content).to include('Name')
    end
  end
end
