# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe MedicationTakePolicy do
  fixtures :all

  subject(:policy) { described_class.new(user, medication_take) }

  let(:admin_user) { users(:admin) }
  let(:doctor_user) { users(:doctor) }
  let(:nurse_user) { users(:nurse) }
  let(:carer_user) { users(:carer) }
  let(:parent_user) { users(:parent) }
  let(:adult_patient_user) { users(:adult_patient) }

  let(:adult_patient) { people(:adult_patient_person) }
  let(:child_patient) { people(:child_user_person) }
  let(:prescription) { prescriptions(:adult_patient_prescription) }
  let(:medication_take) { prescription.medication_takes.build }

  describe '#create?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:create) }
    end

    context 'when user is a doctor' do
      let(:user) { doctor_user }

      it { is_expected.to permit_action(:create) }
    end

    context 'when user is a nurse' do
      let(:user) { nurse_user }

      it { is_expected.to permit_action(:create) }
    end

    context 'when user is a carer' do
      let(:user) { carer_user }

      context 'with assigned patient' do
        let(:prescription) { prescriptions(:child_prescription) }
        let(:medication_take) { prescription.medication_takes.build }

        it { is_expected.to permit_action(:create) }
      end

      context 'with non-assigned patient' do
        let(:prescription) { prescriptions(:adult_patient_prescription) }
        let(:medication_take) { prescription.medication_takes.build }

        it { is_expected.not_to permit_action(:create) }
      end

      context 'with assigned patient person_medicine take' do
        let(:person_medicine) do
          PersonMedicine.new(person: people(:child_user_person), medicine: medicines(:vitamin_d))
        end
        let(:medication_take) { person_medicine.medication_takes.build(taken_at: Time.current) }

        it { is_expected.to permit_action(:create) }
      end

      context 'with non-assigned patient person_medicine take' do
        let(:person_medicine) { person_medicines(:john_vitamin_d) }
        let(:medication_take) { person_medicine.medication_takes.build(taken_at: Time.current) }

        it { is_expected.not_to permit_action(:create) }
      end
    end

    context 'when user is a parent' do
      let(:user) { parent_user }

      context 'with their minor child' do
        let(:prescription) { prescriptions(:child_prescription) }
        let(:medication_take) { prescription.medication_takes.build }

        it { is_expected.to permit_action(:create) }
      end

      context 'with non-child patient' do
        let(:prescription) { prescriptions(:adult_patient_prescription) }
        let(:medication_take) { prescription.medication_takes.build }

        it { is_expected.not_to permit_action(:create) }
      end

      context 'with minor child person_medicine take' do
        let(:person_medicine) do
          PersonMedicine.new(person: people(:child_user_person), medicine: medicines(:vitamin_d))
        end
        let(:medication_take) { person_medicine.medication_takes.build(taken_at: Time.current) }

        it { is_expected.to permit_action(:create) }
      end

      context 'with non-child person_medicine take' do
        let(:person_medicine) { person_medicines(:john_vitamin_d) }
        let(:medication_take) { person_medicine.medication_takes.build(taken_at: Time.current) }

        it { is_expected.not_to permit_action(:create) }
      end
    end

    context 'when user is an adult patient' do
      let(:user) { adult_patient_user }

      context 'with their own prescription' do
        let(:prescription) { prescriptions(:adult_patient_prescription) }
        let(:medication_take) { prescription.medication_takes.build }

        it { is_expected.to permit_action(:create) }
      end

      context 'with another patient prescription' do
        let(:prescription) { prescriptions(:child_prescription) }
        let(:medication_take) { prescription.medication_takes.build }

        it { is_expected.not_to permit_action(:create) }
      end

      context 'with their own person_medicine take' do
        let(:person_medicine) do
          PersonMedicine.new(person: people(:adult_patient_person), medicine: medicines(:vitamin_c))
        end
        let(:medication_take) { person_medicine.medication_takes.build(taken_at: Time.current) }

        it { is_expected.to permit_action(:create) }
      end

      context 'with someone else person_medicine take' do
        let(:person_medicine) { person_medicines(:john_vitamin_d) }
        let(:medication_take) { person_medicine.medication_takes.build(taken_at: Time.current) }

        it { is_expected.not_to permit_action(:create) }
      end
    end

    context 'when user is nil' do
      let(:user) { nil }

      it { is_expected.not_to permit_action(:create) }
    end
  end

  describe '#new?' do
    let(:user) { admin_user }

    it { is_expected.to permit_action(:new) }
  end

  describe 'Scope' do
    describe '#resolve' do
      context 'when user is an administrator' do
        let(:user) { admin_user }

        it 'returns all medication takes' do
          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          expect(scope).to eq(MedicationTake.all)
        end
      end

      context 'when user is a doctor' do
        let(:user) { doctor_user }

        it 'returns all medication takes' do
          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          expect(scope).to eq(MedicationTake.all)
        end
      end

      context 'when user is a nurse' do
        let(:user) { nurse_user }

        it 'returns all medication takes' do
          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          expect(scope).to eq(MedicationTake.all)
        end
      end

      context 'when user is a carer' do
        let(:user) { carer_user }

        it 'returns medication takes for assigned patients only, including person_medicine takes' do
          patient_ids = user.person.patient_ids
          pm = create(:person_medicine, person: people(:child_user_person), medicine: medicines(:aspirin))
          pm_take = create(:medication_take, :for_person_medicine, person_medicine: pm)

          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          prescription_takes = MedicationTake.joins(:prescription)
                                             .where(prescriptions: { person_id: patient_ids })
          person_medicine_takes = MedicationTake.joins(:person_medicine)
                                                .where(person_medicines: { person_id: patient_ids })
          expected_ids = (prescription_takes.pluck(:id) + person_medicine_takes.pluck(:id)).uniq.sort

          expect(scope.pluck(:id).sort).to eq(expected_ids)
          expect(scope.pluck(:id)).to include(pm_take.id)
        end
      end

      context 'when user is a parent' do
        let(:user) { parent_user }

        it 'returns medication takes for their minor children only, including person_medicine takes' do
          child_ids = Person.where(id: user.person.patient_ids, person_type: :minor).pluck(:id)
          pm = create(:person_medicine, person: people(:child_user_person), medicine: medicines(:aspirin))
          pm_take = create(:medication_take, :for_person_medicine, person_medicine: pm)

          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          prescription_takes = MedicationTake.joins(:prescription)
                                             .where(prescriptions: { person_id: child_ids })
          person_medicine_takes = MedicationTake.joins(:person_medicine)
                                                .where(person_medicines: { person_id: child_ids })
          expected_ids = (prescription_takes.pluck(:id) + person_medicine_takes.pluck(:id)).uniq.sort

          expect(scope.pluck(:id).sort).to eq(expected_ids)
          expect(scope.pluck(:id)).to include(pm_take.id)
        end
      end

      context 'when user is an adult patient' do
        let(:user) { adult_patient_user }

        it 'returns only their own medication takes, including person_medicine takes' do
          pm = create(:person_medicine, person: people(:adult_patient_person), medicine: medicines(:aspirin))
          pm_take = create(:medication_take, :for_person_medicine, person_medicine: pm)

          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          prescription_takes = MedicationTake.joins(:prescription)
                                             .where(prescriptions: { person_id: user.person.id })
          person_medicine_takes = MedicationTake.joins(:person_medicine)
                                                .where(person_medicines: { person_id: user.person.id })
          expected_ids = (prescription_takes.pluck(:id) + person_medicine_takes.pluck(:id)).uniq.sort

          expect(scope.pluck(:id).sort).to eq(expected_ids)
          expect(scope.pluck(:id)).to include(pm_take.id)
        end
      end

      context 'when user is nil' do
        let(:user) { nil }

        it 'returns no medication takes' do
          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          expect(scope).to be_empty
        end
      end

      context 'when user has no person' do
        let(:user) { users(:admin) }

        before do
          allow(user).to receive(:person).and_return(nil)
        end

        it 'returns all medication takes for admin' do
          scope = described_class::Scope.new(user, MedicationTake.all).resolve
          expect(scope).to eq(MedicationTake.all)
        end
      end
    end
  end
end
