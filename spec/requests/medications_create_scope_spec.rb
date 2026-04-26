# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication creation scope' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :carer_relationships

  let(:parent_user) { users(:jane) }
  let!(:foreign_location) { locations(:grandmas) }

  before { sign_in(parent_user) }

  describe 'GET /medications/new' do
    it 'renders the guided dose schedule step separately from stock setup' do
      get new_medication_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Details')
      expect(response.body).to include('Dose')
      expect(response.body).to include('Supply')
      expect(response.body).to include('Warnings')
      expect(response.body).to include('data-controller="medication-schedule-wizard"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][amount]"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][unit]"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][frequency]"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][default_max_daily_doses]"')
      expect(response.body).to include(
        'name="medication[dosage_records_attributes][0][default_min_hours_between_doses]"'
      )
      expect(response.body).to include('name="medication[dosage_records_attributes][0][default_dose_cycle]"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][default_for_adults]"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][default_for_children]"')
      expect(response.body).to include('name="onboarding_schedule[person_id]"')
      expect(response.body).to include('name="onboarding_schedule[schedule_type]"')
      expect(response.body).to include('name="onboarding_schedule[frequency]"')
      expect(response.body).to include('name="onboarding_schedule[start_date]"')
      expect(response.body).to include('name="onboarding_schedule[end_date]"')
      expect(response.body).to include('name="onboarding_schedule[max_daily_doses]"')
      expect(response.body).to include('name="onboarding_schedule[min_hours_between_doses]"')
      expect(response.body).to include('name="onboarding_schedule[dose_cycle]"')
      expect(response.body).to include('name="onboarding_schedule[schedule_config]"')
      expect(response.body).to include('Multiple daily')
      expect(response.body).to include('Specific dates')
      expect(response.body).to include('As needed')
      expect(response.body).to include('Tapering')
      expect(response.body).to include('data-medication-schedule-wizard-target="personSelect"')
      expect(response.body).to include('grid-cols-[minmax(0,1fr)_minmax(6.5rem,10rem)]')
      expect(response.body).to include('Add date')
      expect(response.body).to include('Review dose schedule')
      expect(response.body).to include('name="medication[current_supply]"')
      expect(response.body).to include('name="medication[reorder_threshold]"')
      expect(response.body).to include('Supply setup')
      expect(response.body).not_to include('Dosage &amp; Supply')
    end

    it 'shows only authorized locations in the form' do
      get new_medication_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(locations(:home).name)
      expect(response.body).to include(locations(:school).name)
      expect(response.body).not_to include(foreign_location.name)
    end

    it 'prefills the medication name and barcode from the finder selection' do
      get new_medication_path, params: {
        name: 'Laxido Orange oral powder sachets (Galen Ltd)',
        barcode: '5016298210989',
        dmd_code: '13629411000001105',
        dmd_system: 'https://dmd.nhs.uk',
        dmd_concept_class: 'AMPP'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-testid="medication-wizard-form"')
      expect(response.body).to include('value="Laxido Orange oral powder sachets (Galen Ltd)"')
      expect(response.body).to include('name="medication[barcode]"')
      expect(response.body).to include('value="5016298210989"')
      expect(response.body).to include('name="medication[dmd_code]"')
      expect(response.body).to include('value="13629411000001105"')
      expect(response.body).to include('name="medication[dmd_system]"')
      expect(response.body).to include('name="medication[dmd_concept_class]"')
    end

    it 'prefills onboarding defaults from curated dm+d barcode metadata' do
      get new_medication_path, params: {
        name: 'Pregnacare Plus tablets and capsules (Vitabiotics Ltd)',
        barcode: '5021265232062',
        dmd_code: '35394411000001103',
        dmd_system: 'https://dmd.nhs.uk',
        dmd_concept_class: 'AMPP'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('value="Pregnacare Plus tablets and capsules (Vitabiotics Ltd)"')
      expect(response.body).to include('name="medication[current_supply]"')
      expect(response.body).to include('value="84"')
      expect(response.body).to include('name="medication[reorder_threshold]"')
      expect(response.body).to include('value="21"')
    end

    it 'prefills onboarding defaults from curated refill-product barcode metadata' do
      get new_medication_path, params: {
        name: 'Calpol Vapour Plug & Nightlight + 3 Refill Pads',
        barcode: '3574661646435'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('value="Calpol Vapour Plug & Nightlight + 3 Refill Pads"')
      expect(response.body).to include('name="medication[current_supply]"')
      expect(response.body).to include('value="3"')
      expect(response.body).to include('name="medication[reorder_threshold]"')
      expect(response.body).to include('value="0"')
    end

    it 'prefills richer onboarding defaults for Calpol Six Plus oral suspension' do
      get new_medication_path, params: {
        name: 'Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)',
        dmd_code: '316811000001106',
        dmd_system: 'https://dmd.nhs.uk',
        dmd_concept_class: 'AMP'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)')
      expect(response.body).to include('Analgesic')
      expect(response.body).to include('mild to moderate pain')
      expect(response.body).to include('Contains paracetamol')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][unit]"')
      expect(response.body).to include('value="ml"')
      expect(response.body).to include('name="medication[dosage_records_attributes][0][amount]"')
      expect(response.body).to include('Children 6-8 years')
      expect(response.body).to include('Adults and children over 16 years')
    end

    it 'ignores an invalid non-GTIN barcode from the finder selection' do
      get new_medication_path, params: {
        name: 'Aspirin 300mg tablets',
        barcode: '1234567890'
      }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-testid="medication-wizard-form"')
      expect(response.body).to include('value="Aspirin 300mg tablets"')
      expect(response.body).not_to include('name="medication[barcode]"')
      expect(response.body).not_to include('value="1234567890"')
    end

    it 'prefills clean supplement onboarding data from Open Food Facts metadata' do
      get new_medication_path, params: {
        name: 'Wellman Original',
        description: 'Daily multivitamin food supplement',
        barcode: '5021265221301',
        category: 'Supplement',
        package_quantity: '29',
        package_unit: 'tablet'
      }

      html = Nokogiri::HTML(response.body)
      selected_category = html.at_css('input[name="medication[category]"][value="Supplement"]')
      description = html.at_css('textarea[name="medication[description]"]')
      amount = html.at_css("input[name='medication[dosage_records_attributes][0][amount]']")
      unit = html.at_css("input[name='medication[dosage_records_attributes][0][unit]'][value='tablet']")

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('value="Wellman Original"')
      expect(response.body).not_to include('Wellman Original (Wellman) 29')
      expect(description.text).to eq('Daily multivitamin food supplement')
      expect(selected_category).to be_present
      expect(selected_category['checked']).to be_present
      expect(response.body).to include('name="medication[current_supply]"')
      expect(response.body).to include('value="29"')
      expect(amount['value']).to eq('1.0')
      expect(unit).to be_present
    end

    it 'looks up Open Food Facts supplement metadata when only the barcode is supplied' do
      open_food_facts_lookup = instance_double(
        OpenFoodFacts::BarcodeLookup,
        lookup: {
          name: 'Wellman Original',
          description: 'Daily multivitamin food supplement',
          category: 'Supplement',
          package_quantity: 30,
          package_unit: 'tablet'
        }
      )
      allow(OpenFoodFacts::BarcodeLookup).to receive(:new).and_return(open_food_facts_lookup)

      get new_medication_path, params: { barcode: '5021265221301' }

      html = Nokogiri::HTML(response.body)
      description = html.at_css('textarea[name="medication[description]"]')
      amount = html.at_css("input[name='medication[dosage_records_attributes][0][amount]']")
      unit = html.at_css("input[name='medication[dosage_records_attributes][0][unit]'][value='tablet']")

      expect(response).to have_http_status(:ok)
      expect(open_food_facts_lookup).to have_received(:lookup).with('5021265221301')
      expect(response.body).to include('value="Wellman Original"')
      expect(description.text).to eq('Daily multivitamin food supplement')
      expect(response.body).to include('value="30"')
      expect(amount['value']).to eq('1.0')
      expect(unit).to be_present
    end
  end

  describe 'POST /medications' do
    it 'creates a medication in an authorized location' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Scoped Parent Medication',
            category: 'Vitamin',
            dosage_amount: 5,
            dosage_unit: 'ml',
            current_supply: 10,
            reorder_threshold: 1,
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      expect(response).to redirect_to(medication_path(Medication.last))
      expect(Medication.last.location).to eq(locations(:home))
    end

    it 'persists a barcode selected from the finder flow' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Laxido Orange oral powder sachets (Galen Ltd)',
            barcode: '5016298210989',
            dmd_code: '13629411000001105',
            dmd_system: 'https://dmd.nhs.uk',
            dmd_concept_class: 'AMPP',
            category: 'Osmotic Laxative',
            dosage_amount: 1,
            dosage_unit: 'sachet',
            current_supply: 30,
            reorder_threshold: 5,
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      expect(Medication.last.barcode).to eq('5016298210989')
      expect(Medication.last.dmd_code).to eq('13629411000001105')
      expect(Medication.last.dmd_system).to eq('https://dmd.nhs.uk')
      expect(Medication.last.dmd_concept_class).to eq('AMPP')
    end

    it 'fills missing onboarding fields and suggested doses from curated dm+d metadata' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Pregnacare Plus tablets and capsules (Vitabiotics Ltd)',
            barcode: '5021265232062',
            dmd_code: '35394411000001103',
            dmd_system: 'https://dmd.nhs.uk',
            dmd_concept_class: 'AMPP',
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      medication = Medication.last

      expect(medication).to have_attributes(
        current_supply: 84,
        reorder_threshold: 21,
        barcode: '5021265232062',
        dmd_code: '35394411000001103'
      )
      expect(medication.dosage_records.order(:id).pluck(:amount, :unit, :current_supply, :reorder_threshold)).to eq(
        [
          [BigDecimal('1.0'), 'tablet', 56, 14],
          [BigDecimal('1.0'), 'capsule', 28, 7]
        ]
      )
    end

    it 'fills missing onboarding fields and suggested doses from curated refill-product metadata' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Calpol Vapour Plug & Nightlight + 3 Refill Pads',
            barcode: '3574661646435',
            category: 'Supplement',
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      medication = Medication.last

      expect(medication).to have_attributes(
        current_supply: 3,
        reorder_threshold: 0,
        barcode: '3574661646435',
        dosage_amount: nil,
        dosage_unit: 'pad',
        dmd_code: nil
      )
      expect(medication.dosage_records.order(:id).pluck(:amount, :unit, :current_supply, :reorder_threshold)).to eq(
        [
          [BigDecimal('1.0'), 'pad', 3, 0]
        ]
      )
    end

    it 'fills rich onboarding defaults and suggested doses for Calpol Six Plus oral suspension' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)',
            dmd_code: '316811000001106',
            dmd_system: 'https://dmd.nhs.uk',
            dmd_concept_class: 'AMP',
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      medication = Medication.last

      expect(medication).to have_attributes(
        category: 'Analgesic',
        description: a_string_including('mild to moderate pain'),
        warnings: a_string_including('Contains paracetamol'),
        dmd_code: '316811000001106',
        dosage_unit: 'ml',
        current_supply: nil,
        reorder_threshold: 0
      )
      dose_tuples = medication.dosage_records.order(:amount, :id).pluck(:amount, :unit, :description)
      dose_rows = dose_tuples.map do |amount, unit, description|
        [amount.to_s('F'), unit, description]
      end

      expect(dose_rows).to include(
        ['5.0', 'ml', 'Children 6-8 years'],
        ['7.5', 'ml', 'Children 8-10 years'],
        ['10.0', 'ml', 'Children 10-12 years'],
        ['10.0', 'ml', 'Children 12-16 years'],
        ['15.0', 'ml', 'Children 12-16 years'],
        ['10.0', 'ml', 'Adults and children over 16 years'],
        ['20.0', 'ml', 'Adults and children over 16 years']
      )
    end

    it 'does not overwrite explicit onboarding values when dm+d defaults are available' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Pregnacare Plus tablets and capsules (Vitabiotics Ltd)',
            barcode: '5021265232062',
            dmd_code: '35394411000001103',
            dmd_system: 'https://dmd.nhs.uk',
            dmd_concept_class: 'AMPP',
            current_supply: 100,
            reorder_threshold: 30,
            location_id: locations(:home).id
          }
        }
      end.to change(Medication, :count).by(1)

      medication = Medication.last

      expect(medication.current_supply).to eq(100)
      expect(medication.reorder_threshold).to eq(30)
    end

    it 'shows a friendly error when the barcode is already used in another inaccessible inventory item' do
      create(:medication, barcode: '5016298210989', location: foreign_location)

      expect do
        post medications_path, params: {
          medication: {
            name: 'Laxido Orange oral powder sachets (Galen Ltd)',
            barcode: '5016298210989',
            category: 'Osmotic Laxative',
            dosage_amount: 1,
            dosage_unit: 'sachet',
            current_supply: 30,
            reorder_threshold: 5,
            location_id: locations(:home).id
          }
        }
      end.not_to change(Medication, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Barcode is already linked to another medication in inventory')
      expect(response.body).not_to include('has already been taken')
    end

    it 'rejects a forged foreign location_id' do
      expect do
        post medications_path, params: {
          medication: {
            name: 'Foreign Location Medication',
            category: 'Vitamin',
            dosage_amount: 5,
            dosage_unit: 'ml',
            current_supply: 10,
            reorder_threshold: 1,
            location_id: foreign_location.id
          }
        }
      end.not_to change(Medication, :count)

      expect(response).to redirect_to(root_path)
    end
  end
end
