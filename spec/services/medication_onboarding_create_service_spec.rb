# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationOnboardingCreateService do
  let(:location) { create(:location) }

  def build_medication(attrs = {})
    build(:medication, location: location, **attrs)
  end

  def create_medication(attrs = {})
    create(:medication, location: location, **attrs)
  end

  def medication_with_dosage(attrs = {})
    med = build_medication(attrs)
    med.dosage_records.build(amount: 500, unit: 'mg', frequency: 'As needed',
                             default_max_daily_doses: 2, default_min_hours_between_doses: 8,
                             default_dose_cycle: 'daily', default_for_adults: true)
    med
  end

  def call_service(
    medication:,
    schedule_attributes: nil,
    people_scope: nil,
    medication_scope: nil,
    plan_authorizer: ->(_record) {}
  )
    described_class.new(
      medication: medication,
      schedule_attributes: schedule_attributes,
      people_scope: people_scope,
      medication_scope: medication_scope,
      plan_authorizer: plan_authorizer
    ).call
  end

  describe 'result shape' do
    it 'exposes success, medication, schedule, and restocked attributes' do
      medication = build_medication
      result = call_service(medication: medication)
      expect(result).to respond_to(:success, :medication, :schedule, :restocked)
    end

    it 'exposes a restocked? predicate' do
      medication = build_medication
      result = call_service(medication: medication)
      expect(result).to respond_to(:restocked?)
    end
  end

  describe '#call — no schedule requested' do
    it 'persists the medication' do
      medication = build_medication
      expect { call_service(medication: medication) }
        .to change(Medication, :count).by(1)
    end

    it 'returns success: true' do
      medication = build_medication
      result = call_service(medication: medication)
      expect(result.success).to be(true)
    end

    it 'returns the medication in the result' do
      medication = build_medication
      result = call_service(medication: medication)
      expect(result.medication).to eq(medication)
    end

    it 'returns schedule: nil' do
      medication = build_medication
      result = call_service(medication: medication)
      expect(result.schedule).to be_nil
    end

    it 'returns restocked: false' do
      medication = build_medication
      result = call_service(medication: medication)
      expect(result.restocked?).to be(false)
    end

    it 'sets the paper_trail_event to create' do
      medication = build_medication
      allow(medication).to receive(:paper_trail_event=).with('create').and_call_original
      call_service(medication: medication)
      expect(medication).to have_received(:paper_trail_event=).with('create')
    end

    context 'when the medication fails validation' do
      it 'returns success: false' do
        medication = build(:medication, location: nil, name: '')
        result = call_service(medication: medication)
        expect(result.success).to be(false)
      end

      it 'does not persist the medication' do
        medication = build(:medication, location: nil, name: '')
        expect { call_service(medication: medication) }
          .not_to change(Medication, :count)
      end
    end
  end

  describe '#call — with schedule_attributes and people_scope' do
    let(:person) { create(:person) }
    let(:people_scope) { Person.where(id: person.id) }

    def schedule_attrs(type: 'multiple_daily', extra: {})
      {
        person_id: person.id,
        schedule_type: type,
        max_daily_doses: 2,
        min_hours_between_doses: 8,
        start_date: Time.zone.today,
        end_date: 1.month.from_now.to_date
      }.merge(extra)
    end

    it 'persists the medication' do
      medication = medication_with_dosage
      expect { call_service(medication: medication, schedule_attributes: schedule_attrs, people_scope: people_scope) }
        .to change(Medication, :count).by(1)
    end

    it 'persists a Schedule record' do
      medication = medication_with_dosage
      expect { call_service(medication: medication, schedule_attributes: schedule_attrs, people_scope: people_scope) }
        .to change(Schedule, :count).by(1)
    end

    it 'requires a plan authorizer before saving a Schedule record' do
      medication = medication_with_dosage
      captured_error = nil

      expect do
        call_service(
          medication: medication,
          schedule_attributes: schedule_attrs,
          people_scope: people_scope,
          plan_authorizer: nil
        )
      rescue StandardError => e
        captured_error = e
      end.not_to change(Schedule, :count)

      expect(captured_error.class.name).to eq('MedicationOnboardingCreateService::MissingPlanAuthorizer')
    end

    it 'returns success: true' do
      medication = medication_with_dosage
      result = call_service(medication: medication, schedule_attributes: schedule_attrs, people_scope: people_scope)
      expect(result.success).to be(true)
    end

    it 'returns the created schedule in the result' do
      medication = medication_with_dosage
      result = call_service(medication: medication, schedule_attributes: schedule_attrs, people_scope: people_scope)
      expect(result.schedule).to be_a(Schedule)
      expect(result.schedule).to be_persisted
    end

    it 'assigns default_schedule_type from schedule_attributes' do
      medication = medication_with_dosage
      attrs = schedule_attrs(type: 'multiple_daily')
      call_service(medication: medication, schedule_attributes: attrs, people_scope: people_scope)
      expect(medication.default_schedule_type).to eq('multiple_daily')
    end

    it 'returns restocked: false' do
      medication = medication_with_dosage
      result = call_service(medication: medication, schedule_attributes: schedule_attrs, people_scope: people_scope)
      expect(result.restocked?).to be(false)
    end

    context 'when schedule_config is a hash' do
      it 'stores schedule_config on the medication' do
        medication = medication_with_dosage
        config = { 'times' => %w[08:00 20:00] }
        call_service(
          medication: medication,
          schedule_attributes: schedule_attrs(extra: { schedule_config: config }),
          people_scope: people_scope
        )
        expect(medication.reload.default_schedule_config).to eq('times' => %w[08:00 20:00])
      end
    end

    context 'when the plan record fails to save (rolls back)' do
      it 'rolls back the medication creation when person_id is invalid' do
        medication = medication_with_dosage
        bad_attrs = schedule_attrs.merge(person_id: -1)
        expect do
          call_service(medication: medication, schedule_attributes: bad_attrs, people_scope: people_scope)
        rescue ActiveRecord::RecordNotFound
          nil
        end.not_to change(Medication, :count)
      end
    end

    context 'when medication itself fails validation inside transaction' do
      it 'returns success: false' do
        medication = build(:medication, location: nil, name: '')
        result = call_service(
          medication: medication,
          schedule_attributes: schedule_attrs,
          people_scope: people_scope
        )
        expect(result.success).to be(false)
      end

      it 'resets record ids so it can be resubmitted' do
        medication = build(:medication, location: nil, name: '')
        call_service(
          medication: medication,
          schedule_attributes: schedule_attrs,
          people_scope: people_scope
        )
        expect(medication.id).to be_nil
        expect(medication).to be_new_record
      end
    end

    context 'with a PRN schedule type (results in PersonMedication)' do
      it 'does NOT create a Schedule' do
        medication = medication_with_dosage(category: 'Analgesic')
        schedule_count = Schedule.count
        call_service(
          medication: medication,
          schedule_attributes: schedule_attrs(type: 'prn'),
          people_scope: people_scope
        )
        expect(Schedule.count).to eq(schedule_count)
      end

      it 'creates a PersonMedication' do
        medication = medication_with_dosage(category: 'Analgesic')
        expect do
          call_service(
            medication: medication,
            schedule_attributes: schedule_attrs(type: 'prn'),
            people_scope: people_scope
          )
        end.to change(PersonMedication, :count).by(1)
      end

      it 'requires a plan authorizer before saving a PersonMedication record' do
        medication = medication_with_dosage(category: 'Analgesic')
        captured_error = nil

        expect do
          call_service(
            medication: medication,
            schedule_attributes: schedule_attrs(type: 'prn'),
            people_scope: people_scope,
            plan_authorizer: nil
          )
        rescue StandardError => e
          captured_error = e
        end.not_to change(PersonMedication, :count)

        expect(captured_error.class.name).to eq('MedicationOnboardingCreateService::MissingPlanAuthorizer')
      end

      it 'returns nil for schedule in the result (PersonMedication is not a Schedule)' do
        medication = medication_with_dosage(category: 'Analgesic')
        result = call_service(
          medication: medication,
          schedule_attributes: schedule_attrs(type: 'prn'),
          people_scope: people_scope
        )
        expect(result.schedule).to be_nil
      end
    end

    context 'when schedule_attributes present but people_scope is nil' do
      it 'treats it as no schedule requested and only saves the medication' do
        medication = build_medication
        schedule_count_before = Schedule.count
        expect do
          call_service(medication: medication, schedule_attributes: schedule_attrs, people_scope: nil)
        end.to change(Medication, :count).by(1)
        expect(Schedule.count).to eq(schedule_count_before)
      end
    end
  end

  describe '#call — stock merge (restock existing medication)' do
    let(:existing_medication) do
      create_medication(
        name: 'Paracetamol 500mg tablets',
        current_supply: 10,
        dosage_amount: 500,
        dosage_unit: 'mg'
      )
    end
    let(:medication_scope) { Medication.where(location: location) }

    before do
      # Ensure the existing medication is in the scope
      existing_medication
    end

    context 'when incoming medication has current_supply and matches an existing one' do
      it 'does not create a new medication' do
        incoming = build_medication(name: existing_medication.name, current_supply: 5)
        expect { call_service(medication: incoming, medication_scope: medication_scope) }
          .not_to change(Medication, :count)
      end

      it 'restocks the existing medication' do
        incoming = build_medication(name: existing_medication.name, current_supply: 5)
        call_service(medication: incoming, medication_scope: medication_scope)
        expect(existing_medication.reload.current_supply).to eq(15)
      end

      it 'returns restocked: true' do
        incoming = build_medication(name: existing_medication.name, current_supply: 5)
        result = call_service(medication: incoming, medication_scope: medication_scope)
        expect(result.restocked?).to be(true)
      end

      it 'returns the existing medication in the result' do
        incoming = build_medication(name: existing_medication.name, current_supply: 5)
        result = call_service(medication: incoming, medication_scope: medication_scope)
        expect(result.medication).to eq(existing_medication)
      end

      it 'returns success: true' do
        incoming = build_medication(name: existing_medication.name, current_supply: 5)
        result = call_service(medication: incoming, medication_scope: medication_scope)
        expect(result.success).to be(true)
      end
    end

    context 'when incoming medication has zero current_supply and no tracked dosages' do
      it 'creates a new medication (not a merge candidate)' do
        incoming = build_medication(name: existing_medication.name, current_supply: 0)
        expect { call_service(medication: incoming, medication_scope: medication_scope) }
          .to change(Medication, :count).by(1)
      end
    end

    context 'when medication_scope is nil' do
      it 'creates a new medication even if current_supply is present' do
        incoming = build_medication(name: existing_medication.name, current_supply: 5)
        expect { call_service(medication: incoming, medication_scope: nil) }
          .to change(Medication, :count).by(1)
      end
    end
  end

  describe 'normalized_schedule_config' do
    let(:person) { create(:person) }
    let(:people_scope) { Person.where(id: person.id) }

    def schedule_config_attrs(config)
      {
        person_id: person.id,
        schedule_type: 'multiple_daily',
        max_daily_doses: 1,
        min_hours_between_doses: 24,
        start_date: Time.zone.today,
        end_date: 1.month.from_now.to_date,
        schedule_config: config
      }
    end

    def call_with_config(config)
      medication = medication_with_dosage
      call_service(medication: medication, schedule_attributes: schedule_config_attrs(config),
                   people_scope: people_scope)
      medication
    end

    it 'accepts a plain hash for schedule_config' do
      med = call_with_config({ 'times' => %w[09:00] })
      expect(med.reload.default_schedule_config).to eq('times' => %w[09:00])
    end

    it 'returns empty hash when schedule_config is blank' do
      med = call_with_config(nil)
      expect(med.reload.default_schedule_config).to eq({})
    end

    it 'returns empty hash when schedule_config is invalid JSON string' do
      med = call_with_config('not-json{')
      expect(med.reload.default_schedule_config).to eq({})
    end
  end
end
