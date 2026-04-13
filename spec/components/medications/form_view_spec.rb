# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::FormView, type: :component do
  def build_dose_option(medication, persisted: false, **overrides)
    attributes = {
      amount: 5,
      unit: 'ml',
      frequency: 'Once daily',
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    }.merge(overrides)

    persisted ? medication.dosage_records.create!(attributes) : medication.dosage_records.build(attributes)
  end

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

  describe 'dosage input constraints' do
    it 'renders dosage amount input with minimum value of 1' do
      medication = Medication.new(name: 'Test Medication', dosage_amount: 1)
      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      dosage_input = rendered.at_css("input#medication_dosage_amount[name='medication[dosage_amount]']")
      expect(dosage_input).not_to be_nil
      expect(dosage_input['min']).to eq('1')
    end
  end

  describe 'dose options management' do
    it 'renders nested medication-owned dosage fields' do
      medication = build(:medication)
      build_dose_option(medication)

      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))
      html = rendered.to_html

      expect(html).to include(
        'Dose Options',
        'Manage all medication-owned dose options here.',
        'name="medication[dosage_records_attributes][0][amount]"',
        'name="medication[dosage_records_attributes][0][unit]"',
        'name="medication[dosage_records_attributes][0][frequency]"',
        'Matches the main dose unit'
      )
    end

    it 'renders frequency suggestions for medication-owned dosage fields' do
      medication = build(:medication)
      build_dose_option(medication)

      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))
      html = rendered.to_html

      expect(html).to match(/data-controller="[^"]*frequency-suggestions[^"]*"/)
      expect(html).to include(
        'data-action="click-&gt;frequency-suggestions#suggest"',
        'Every morning',
        'Every evening',
        'Once weekly'
      )
      expect(html).not_to include('As needed (PRN)')
    end

    it 'renders structured frequency template payloads for the suggestion chips' do
      medication = build(:medication)
      build_dose_option(medication)

      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      morning_chip = rendered.at_css("button[data-frequency='Every morning']")
      expect(morning_chip).not_to be_nil
      expect(morning_chip['data-frequency-suggestions-frequency-value']).to eq('Every morning')
      expect(morning_chip['data-frequency-suggestions-max-doses-value']).to eq('1')
      expect(morning_chip['data-frequency-suggestions-min-hours-value']).to eq('24')
      expect(morning_chip['data-frequency-suggestions-dose-cycle-value']).to eq('daily')
    end

    it 'marks timing defaults as required fields' do
      medication = build(:medication)
      build_dose_option(medication)

      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      max_doses = rendered.at_css("input[name='medication[dosage_records_attributes][0][default_max_daily_doses]']")
      min_hours = rendered.at_css(
        "input[name='medication[dosage_records_attributes][0][default_min_hours_between_doses]']"
      )
      dose_cycle = rendered.at_css("select[name='medication[dosage_records_attributes][0][default_dose_cycle]']")

      expect(max_doses.attribute('required')).not_to be_nil
      expect(min_hours.attribute('required')).not_to be_nil
      expect(dose_cycle.attribute('required')).not_to be_nil
    end

    it 'renders destructive remove controls instead of a checkbox' do
      medication = create(:medication)
      build_dose_option(medication, persisted: true)

      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))
      html = rendered.to_html

      expect(html).to include('Remove dose option')
      expect(html).to include('Undo')
      expect(rendered.css("button[data-action='click->dosage-options#remove']")).to be_present
      expect(rendered.css("button[data-action='click->dosage-options#undo']")).to be_present
      expect(
        rendered.css("input[type='checkbox'][name='medication[dosage_records_attributes][0][_destroy]']")
      ).to be_empty
    end

    it 'greys out dose option units and keeps them aligned with the main medication unit' do
      medication = build(:medication, dosage_unit: 'ml')
      build_dose_option(medication, amount: 2.5, unit: 'mg', frequency: 'Every morning')

      rendered = render_inline(described_class.new(medication: medication, title: 'Test Title'))

      hidden_unit = rendered.at_css("input[type='hidden'][name='medication[dosage_records_attributes][0][unit]']")
      display_unit = rendered.at_css('#medication_dosage_records_attributes_0_display_unit')

      expect(hidden_unit['value']).to eq('ml')
      expect(display_unit['value']).to eq('ml')
      expect(display_unit['disabled']).to eq('disabled')
      expect(display_unit['class']).to include('bg-muted/70')
    end
  end
end
