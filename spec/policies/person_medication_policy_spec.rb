# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe PersonMedicationPolicy do
  fixtures :all

  subject(:policy) { described_class.new(user, person_medication) }

  let(:admin_user) { users(:admin) }
  let(:doctor_user) { users(:doctor) }
  let(:nurse_user) { users(:nurse) }
  let(:carer_user) { users(:carer) }
  let(:parent_user) { users(:parent) }
  let(:adult_patient_user) { users(:adult_patient) }

  let(:adult_patient) { people(:adult_patient_person) }
  let(:child_patient) { people(:child_user_person) }
  let(:person_medication) { person_medications(:john_vitamin_d) }

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
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:show) }
    end

    context 'when user is a carer without assigned patient' do
      let(:user) { carer_user }
      let(:person_medication) { PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_d)) }

      it { is_expected.not_to permit_action(:show) }
    end

    context 'when user views their own person medication' do
      let(:user) { adult_patient_user }
      let(:person_medication) { PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_d)) }

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
      let(:person_medication) { PersonMedication.new(person: adult_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:create) }
    end

    context 'when parent creates for their child' do
      let(:user) { parent_user }
      let(:person_medication) { PersonMedication.new(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:create) }
    end

    context 'when carer creates for assigned patient' do
      let(:user) { carer_user }
      let(:person_medication) { PersonMedication.new(person: child_patient, medication: medications(:vitamin_d)) }

      # Carers cannot add medications for their patients - only admins can
      it { is_expected.not_to permit_action(:create) }
    end

    context 'when user creates for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medication) { PersonMedication.new(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.not_to permit_action(:create) }
    end

    context 'when user is nil' do
      let(:user) { nil }

      it { is_expected.not_to permit_action(:create) }
    end
  end

  describe '#new?' do
    # new? is called with the class PersonMedication, not an instance
    let(:person_medication) { PersonMedication }

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
      let(:person_medication) { PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:update) }
    end

    context 'when parent updates for their child' do
      let(:user) { parent_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:update) }
    end

    context 'when user updates for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

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
      let(:person_medication) { PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:destroy) }
    end

    context "when parent removes their child's person medication record" do
      let(:user) { parent_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:destroy) }
    end

    context 'when user destroys for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.not_to permit_action(:destroy) }
    end
  end

  describe '#take_medication?' do
    context 'when user is an administrator' do
      let(:user) { admin_user }

      it { is_expected.to permit_action(:take_medication) }
    end

    context 'when user takes their own medication' do
      let(:user) { adult_patient_user }
      let(:person_medication) { PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_d)) }

      it { is_expected.to permit_action(:take_medication) }
    end

    context 'when parent takes medication for their child' do
      let(:user) { parent_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      # Parents can take medication for their children via carer_with_patient?
      it { is_expected.to permit_action(:take_medication) }
    end

    context 'when carer takes medication for assigned patient' do
      let(:user) { carer_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      # Carers can take medication for their patients via carer_with_patient?
      it { is_expected.to permit_action(:take_medication) }
    end

    context 'when user takes medication for unrelated person' do
      let(:user) { adult_patient_user }
      let(:person_medication) { PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d)) }

      it { is_expected.not_to permit_action(:take_medication) }
    end
  end

  describe 'Scope' do
    describe '#resolve' do
      context 'when user is an administrator' do
        let(:user) { admin_user }

        it 'returns all person medications' do
          scope = described_class::Scope.new(user, PersonMedication.all).resolve
          expect(scope).to eq(PersonMedication.all)
        end
      end

      context 'when user is a doctor' do
        let(:user) { doctor_user }

        it 'returns all person medications' do
          scope = described_class::Scope.new(user, PersonMedication.all).resolve
          expect(scope).to eq(PersonMedication.all)
        end
      end

      context 'when user is an adult patient' do
        let(:user) { adult_patient_user }

        before do
          # Create a person medication for the adult patient
          PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_c))
        end

        it 'returns only their own person medications' do
          scope = described_class::Scope.new(user, PersonMedication.all).resolve
          expect(scope.pluck(:person_id).uniq).to eq([user.person_id])
        end
      end

      context 'when user is a parent' do
        let(:user) { parent_user }

        before do
          PersonMedication.create!(person: user.person, medication: medications(:vitamin_c))
          PersonMedication.create!(person: child_patient, medication: medications(:vitamin_d))
          PersonMedication.create!(person: adult_patient, medication: medications(:vitamin_d))
        end

        it 'returns their own and linked child person medications only' do
          scope = described_class::Scope.new(user, PersonMedication.all).resolve
          expect(scope.pluck(:person_id).uniq).to contain_exactly(user.person_id, child_patient.id)
        end
      end

      context 'when user is nil' do
        let(:user) { nil }

        it 'returns no person medications' do
          scope = described_class::Scope.new(user, PersonMedication.all).resolve
          expect(scope).to be_empty
        end
      end
    end
  end
end
