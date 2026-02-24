# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  describe 'i18n translations' do
    around do |example|
      I18n.with_locale(:ga) { example.run }
    end

    it 'renders delete confirmation with Irish translations' do
      medicine = Medicine.new(name: 'Paracetamol')
      person = Person.new(name: 'John')
      prescription = Prescription.new(medicine: medicine, person: person)

      component = described_class.new(prescription: prescription)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Scrios')
      expect(rendered.to_html).to include('Scrios Oideas Leigheasanna?')
      expect(rendered.to_html).to include('Cealaigh')
    end
  end
end
