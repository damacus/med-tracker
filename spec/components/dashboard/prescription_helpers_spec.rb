# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PrescriptionHelpers do
  let(:test_class) do
    Class.new do
      include Components::Dashboard::PrescriptionHelpers

      attr_reader :prescription, :current_user

      def initialize(prescription:, current_user: nil)
        @prescription = prescription
        @current_user = current_user
      end
    end
  end

  let(:prescription) do
    person = create(:person)
    medicine = create(:medicine)
    dosage = Dosage.new(medicine: medicine, amount: 500.0, unit: 'mg')
    Prescription.new(
      person: person,
      medicine: medicine,
      dosage: dosage,
      end_date: Date.new(2024, 12, 31),
      frequency: 'Twice daily'
    )
  end

  let(:user) { instance_double(User, email_address: 'test@example.com', role: :administrator) }
  let(:instance) { test_class.new(prescription: prescription, current_user: user) }

  describe '#format_dosage' do
    context 'when dosage has amount and unit' do
      it 'formats integer amounts without decimal' do
        expect(instance.format_dosage).to eq('500 mg')
      end

      it 'formats decimal amounts with decimal' do
        person = create(:person)
        medicine = create(:medicine)
        dosage = Dosage.new(medicine: medicine, amount: 2.5, unit: 'ml')
        custom_prescription = Prescription.new(person: person, medicine: medicine, dosage: dosage)
        custom_instance = test_class.new(prescription: custom_prescription, current_user: user)

        expect(custom_instance.format_dosage).to eq('2.5 ml')
      end
    end

    context 'when dosage is missing amount' do
      it 'returns em dash' do
        person = create(:person)
        medicine = create(:medicine)
        dosage = Dosage.new(medicine: medicine, amount: nil, unit: 'mg')
        custom_prescription = Prescription.new(person: person, medicine: medicine, dosage: dosage)
        custom_instance = test_class.new(prescription: custom_prescription, current_user: user)

        expect(custom_instance.format_dosage).to eq('—')
      end
    end

    context 'when dosage is missing unit' do
      it 'returns em dash' do
        person = create(:person)
        medicine = create(:medicine)
        dosage = Dosage.new(medicine: medicine, amount: 500.0, unit: nil)
        custom_prescription = Prescription.new(person: person, medicine: medicine, dosage: dosage)
        custom_instance = test_class.new(prescription: custom_prescription, current_user: user)

        expect(custom_instance.format_dosage).to eq('—')
      end
    end

    context 'when dosage is nil' do
      it 'returns em dash' do
        person = create(:person)
        medicine = create(:medicine)
        custom_prescription = Prescription.new(person: person, medicine: medicine, dosage: nil)
        custom_instance = test_class.new(prescription: custom_prescription, current_user: user)

        expect(custom_instance.format_dosage).to eq('—')
      end
    end
  end

  describe '#format_quantity' do
    context 'when medicine has stock' do
      it 'returns the stock as a string' do
        prescription.medicine.stock = 44
        expect(instance.format_quantity).to eq('44')
      end
    end

    context 'when medicine stock is nil' do
      it 'returns em dash' do
        prescription.medicine.stock = nil
        expect(instance.format_quantity).to eq('—')
      end
    end

    context 'when medicine is nil' do
      it 'returns em dash' do
        person = create(:person)
        custom_prescription = Prescription.new(person: person, medicine: nil, dosage: nil)
        custom_instance = test_class.new(prescription: custom_prescription, current_user: user)

        expect(custom_instance.format_quantity).to eq('—')
      end
    end
  end

  describe '#format_end_date' do
    context 'when prescription has end_date' do
      it 'formats the date' do
        prescription.end_date = Date.new(2024, 12, 31)
        expect(instance.format_end_date).to eq('Dec 31, 2024')
      end
    end

    context 'when prescription has no end_date' do
      it 'returns em dash' do
        prescription.end_date = nil
        expect(instance.format_end_date).to eq('—')
      end
    end
  end

  describe '#can_delete?' do
    context 'when current_user is nil' do
      let(:instance) { test_class.new(prescription: prescription, current_user: nil) }

      it 'returns false' do
        expect(instance.can_delete?).to be false
      end
    end

    context 'when current_user is present' do
      it 'delegates to PrescriptionPolicy' do
        policy_double = instance_double(PrescriptionPolicy, destroy?: true)
        allow(PrescriptionPolicy).to receive(:new).with(user, prescription).and_return(policy_double)

        expect(instance.can_delete?).to be true
        expect(PrescriptionPolicy).to have_received(:new).with(user, prescription)
      end

      it 'returns false when policy denies destroy' do
        policy_double = instance_double(PrescriptionPolicy, destroy?: false)
        allow(PrescriptionPolicy).to receive(:new).with(user, prescription).and_return(policy_double)

        expect(instance.can_delete?).to be false
      end
    end
  end
end
