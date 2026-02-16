# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  describe 'i18n translations' do
    it 'renders delete dialog with translated delete button' do
      prescription = instance_double(Prescription, id: 1)
      url_helpers = instance_double(UrlHelpers)

      component = described_class.new(
        prescription: prescription,
        url_helpers: url_helpers
      )

      allow(component).to receive(:t).with('dashboard.delete_confirmation.delete').and_return('Delete')

      render_inline(component)

      expect(rendered_content).to include('Delete')
    end
  end
end
