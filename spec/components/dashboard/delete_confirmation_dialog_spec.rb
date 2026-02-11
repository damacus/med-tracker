# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  let(:prescription) { prescriptions(:active_prescription) }
  let(:url_helpers) { Rails.application.routes.url_helpers }

  describe 'rendering' do
    it 'renders a delete trigger button' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      expect(rendered.text).to include('Delete')
    end

    it 'renders the confirmation dialog content' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      expect(rendered.text).to include('Delete Prescription?')
    end

    it 'includes the medicine name in the confirmation message' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      expect(rendered.text).to include(prescription.medicine.name)
    end

    it 'includes the person name in the confirmation message' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      expect(rendered.text).to include(prescription.person.name)
    end
  end

  describe 'dialog actions' do
    it 'renders a cancel button' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      expect(rendered.text).to include('Cancel')
    end

    it 'renders a delete confirmation form with DELETE method' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      form = rendered.css('form').last
      expect(form).to be_present
    end

    it 'renders a destructive confirm button' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      confirm_button = rendered.css('[data-test-id^="confirm-delete"]')
      expect(confirm_button).to be_present
    end
  end

  describe 'trigger button' do
    it 'renders with a test id based on prescription id' do
      rendered = render_inline(described_class.new(prescription: prescription, url_helpers: url_helpers))

      trigger = rendered.css("[data-test-id='delete-prescription-#{prescription.id}']")
      expect(trigger).to be_present
    end

    it 'applies custom button_class when provided' do
      rendered = render_inline(described_class.new(
                                 prescription: prescription,
                                 url_helpers: url_helpers,
                                 button_class: 'custom-class'
                               ))

      trigger = rendered.css("[data-test-id='delete-prescription-#{prescription.id}']").first
      expect(trigger['class']).to include('custom-class')
    end
  end
end
