# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe PersonMedicinePolicy do
  fixtures :all

  subject(:policy) { described_class.new(user, person_medicine) }

  let(:admin_user) { users(:admin) }
  let(:doctor_user) { users(:doctor) }
  let(:nurse_user) { users(:nurse) }
  let(:carer_user) { users(:carer) }
  let(:parent_user) { users(:parent) }
  let(:adult_patient_user) { users(:adult_patient) }

  let(:adult_patient) { people(:adult_patient_person) }
  let(:child_patient) { people(:child_user_person) }
  let(:person_medicine) { person_medicines(:john_vitamin_d) }

  describe '#show?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:show) }
    end

    context 'when user is a doctor' do
      let(:user) { doctor_user }

      it { is_expected.to permit_action(:show) }
    end

    context 'when user is a nurse' do
      let(:user) { nurse_user }

      it { is_expected.to permit_action(:show) }
    end

    context 'when user is a carer with assigned patient' do
      let(:user) { carer_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.to permit_action(:show) }
    end

    context 'when user is a carer without assigned patient' do
      let(:user) { carer_user }
      let(:person_medicine) { PersonMedicine.create!(person: adult_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.not_to permit_action(:show) }
    end

    context 'when user views their own person medicine' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: adult_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.to permit_action(:show) }
    end
  end

  describe '#create?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:create) }
    end

    context 'when user creates for themselves' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.new(person: adult_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.to permit_action(:create) }
    end

    context 'when parent creates for their child' do
      let(:user) { parent_user }
      let(:person_medicine) { PersonMedicine.new(person: child_patient, medicine: medicines(:vitamin_d)) }

      # Parents cannot add medicines for their children - only admins can
      it { is_expected.not_to permit_action(:create) }
    end

    context 'when carer creates for assigned patient' do
      let(:user) { carer_user }
      let(:person_medicine) { PersonMedicine.new(person: child_patient, medicine: medicines(:vitamin_d)) }

      # Carers cannot add medicines for their patients - only admins can
      it { is_expected.not_to permit_action(:create) }
    end

    context 'when user creates for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.new(person: child_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.not_to permit_action(:create) }
    end

    context 'when user is nil' do
      let(:user) { nil }

      it { is_expected.not_to permit_action(:create) }
    end
  end

  describe '#new?' do
    # new? is called with the class PersonMedicine, not an instance
    let(:person_medicine) { PersonMedicine }

    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:new) }
    end

    context 'when user is a parent' do
      let(:user) { parent_user }

      it { is_expected.to permit_action(:new) }
    end

    context 'when user is an adult patient' do
      let(:user) { adult_patient_user }

      it { is_expected.to permit_action(:new) }
    end
  end

  describe '#update?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:update) }
    end

    context 'when user updates their own' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: adult_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.to permit_action(:update) }
    end

    context 'when parent updates for their child' do
      let(:user) { parent_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      # Parents cannot update medicines for their children - only admins can
      it { is_expected.not_to permit_action(:update) }
    end

    context 'when user updates for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.not_to permit_action(:update) }
    end
  end

  describe '#destroy?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:destroy) }
    end

    context 'when user destroys their own' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: adult_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.to permit_action(:destroy) }
    end

    context 'when user destroys for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.not_to permit_action(:destroy) }
    end
  end

  describe '#take_medicine?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:take_medicine) }
    end

    context 'when user takes their own medicine' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: adult_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.to permit_action(:take_medicine) }
    end

    context 'when parent takes medicine for their child' do
      let(:user) { parent_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      # Parents can take medicine for their children via carer_with_patient?
      it { is_expected.to permit_action(:take_medicine) }
    end

    context 'when carer takes medicine for assigned patient' do
      let(:user) { carer_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      # Carers can take medicine for their patients via carer_with_patient?
      it { is_expected.to permit_action(:take_medicine) }
    end

    context 'when user takes medicine for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medicine) { PersonMedicine.create!(person: child_patient, medicine: medicines(:vitamin_d)) }

      it { is_expected.not_to permit_action(:take_medicine) }
    end
  end

  describe 'Scope' do
    describe '#resolve' do
      context 'when user is an administrator' do
        let(:user) { admin_user }

        it 'returns all person medicines' do
          scope = described_class::Scope.new(user, PersonMedicine.all).resolve
          expect(scope).to eq(PersonMedicine.all)
        end
      end

      context 'when user is a doctor' do
        let(:user) { doctor_user }

        it 'returns all person medicines' do
          scope = described_class::Scope.new(user, PersonMedicine.all).resolve
          expect(scope).to eq(PersonMedicine.all)
        end
      end

      context 'when user is an adult patient' do
        let(:user) { adult_patient_user }

        before do
          # Create a person medicine for the adult patient
          PersonMedicine.create!(person: adult_patient, medicine: medicines(:vitamin_c))
        end

        it 'returns only their own person medicines' do
          scope = described_class::Scope.new(user, PersonMedicine.all).resolve
          expect(scope.pluck(:person_id).uniq).to eq([user.person_id])
        end
      end

      context 'when user is a parent' do
        let(:user) { parent_user }

        before do
          # Create a person medicine for the parent
          PersonMedicine.create!(person: user.person, medicine: medicines(:vitamin_c))
        end

        it 'returns only their own person medicines (not children)' do
          scope = described_class::Scope.new(user, PersonMedicine.all).resolve
          expect(scope.pluck(:person_id).uniq).to eq([user.person_id])
        end
      end

      context 'when user is nil' do
        let(:user) { nil }

        it 'returns no person medicines' do
          scope = described_class::Scope.new(user, PersonMedicine.all).resolve
          expect(scope).to be_empty
        end
      end
    end
  end
end
