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

  describe 'category combobox accessibility and translations' do
    it 'associates category label with the combobox trigger button' do
      medication = Medication.new(name: 'Test Medication')
      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      expect(rendered.css("label[for='medication_category_trigger']")).to be_present
      expect(rendered.css("label[for='medication_category']")).to be_empty
    end

    it 'renders category helper copy from i18n keys' do
      medication = Medication.new(name: 'Test Medication')
      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))
      html = rendered.to_html

      expect(html).to include(I18n.t('forms.medications.filter_categories'))
      expect(html).to include(I18n.t('forms.medications.no_categories_found'))
    end

    it 'renders RubyUI combobox controller and preselects the current category' do
      medication = Medication.new(name: 'Test Medication', category: 'Vitamin')
      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      expect(rendered.css("[data-controller='ruby-ui--combobox']")).to be_present
      expect(rendered.css("input[type='radio'][name='medication[category]'][value='Vitamin'][checked]")).to be_present
    end
  end

  describe 'unit combobox accessibility' do
    it 'associates unit label with the combobox trigger button' do
      medication = Medication.new(name: 'Test Medication')
      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      expect(rendered.css("label[for='medication_dosage_unit_trigger']")).to be_present
      expect(rendered.css("label[for='medication_dosage_unit']")).to be_empty
    end

    it 'renders RubyUI combobox controller and preselects the current dosage unit' do
      medication = Medication.new(name: 'Test Medication', dosage_unit: 'mg')
      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      expect(rendered.css("[data-controller='ruby-ui--combobox']")).to be_present
      expect(rendered.css("input[type='radio'][name='medication[dosage_unit]'][value='mg'][checked]")).to be_present
    end
  end
end
