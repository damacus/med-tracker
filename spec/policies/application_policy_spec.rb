# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApplicationPolicy, type: :policy do
  fixtures :all
  subject(:policy) { described_class.new(users(:admin), :record) }

  it 'denies every default action' do
    aggregate_failures do
      expect(policy.index?).to be(false)
      expect(policy.show?).to be(false)
      expect(policy.create?).to be(false)
      expect(policy.new?).to be(false)
      expect(policy.update?).to be(false)
      expect(policy.edit?).to be(false)
      expect(policy.destroy?).to be(false)
    end
  end

  it 'aliases new? to create? and edit? to update?' do
    expect(policy.method(:new?).original_name).to eq(:create?)
    expect(policy.method(:edit?).original_name).to eq(:update?)
  end

  describe '#person_id_for_authorization' do
    it 'raises NotImplementedError on the abstract base policy' do
      expect { described_class.new(users(:carer), :rec).send(:person_id_for_authorization) }
        .to raise_error(NotImplementedError, /person_id_for_authorization/)
    end
  end

  describe 'authorization predicates (via concrete subclass)' do
    # Concrete policy whose record is the patient Person being authorized against.
    let(:policy_class) do
      Class.new(ApplicationPolicy) do
        def carer_access? = carer_with_patient?
        def parent_access? = parent_with_dependent_patient?

        private

        def person_id_for_authorization = record.id
      end
    end

    def policy_for(user, patient) = policy_class.new(user, patient)

    describe '#carer_with_patient?' do
      let(:patient) { people(:child_patient) }

      it 'is true for a carer with an active relationship to the patient' do
        expect(policy_for(users(:carer), patient).carer_access?).to be(true)
      end

      it 'is false for a carer with no relationship to the patient' do
        expect(policy_for(users(:carer), people(:john)).carer_access?).to be(false)
      end

      it 'is false when the relationship exists but is inactive' do
        # carer_person -> child_patient deactivated: kills the .active drop in
        # the carer predicate (bob's role is carer too, but uses a different person).
        carer_relationships(:carer_cares_for_patient).deactivate!
        expect(policy_for(users(:carer), patient).carer_access?).to be(false)
      end

      it 'is false for a non-carer user' do
        expect(policy_for(users(:parent), patient).carer_access?).to be(false)
      end

      it 'is false for a carer user without a person' do
        carer_without_person = User.new(role: :carer)
        expect(policy_for(carer_without_person, patient).carer_access?).to be(false)
      end

      it 'is false when there is no user' do
        expect(policy_for(nil, patient).carer_access?).to be(false)
      end
    end

    describe '#parent_with_dependent_patient?' do
      let(:dependent_patient) { people(:child_user_person) } # minor, has_capacity false

      it 'is true for a parent with an active relationship to a dependent patient' do
        expect(policy_for(users(:parent), dependent_patient).parent_access?).to be(true)
      end

      it 'is false for a parent when the patient has capacity' do
        capable = people(:child_user_person)
        capable.update!(person_type: :adult, has_capacity: true)
        expect(policy_for(users(:parent), capable).parent_access?).to be(false)
      end

      it 'is false for an incapacitated adult patient (person_type filter excludes plain adults)' do
        # Capacity false but person_type adult: distinguishes the person_type
        # filter from the has_capacity filter.
        people(:child_user_person).update!(person_type: :adult, has_capacity: false)
        expect(policy_for(users(:parent), people(:child_user_person)).parent_access?).to be(false)
      end

      it 'is false when the relationship to the dependent patient is inactive' do
        # Deactivate the only active parent relationship: kills the .active drop.
        carer_relationships(:parent_cares_for_child).deactivate!
        expect(policy_for(users(:parent), dependent_patient).parent_access?).to be(false)
      end

      it 'is false for a parent with no relationship to the patient' do
        expect(policy_for(users(:parent), people(:john)).parent_access?).to be(false)
      end

      it 'is false for a non-parent user' do
        expect(policy_for(users(:carer), dependent_patient).parent_access?).to be(false)
      end

      it 'is false for a parent user without a person' do
        parent_without_person = User.new(role: :parent)
        expect(policy_for(parent_without_person, dependent_patient).parent_access?).to be(false)
      end

      it 'is false when there is no user' do
        expect(policy_for(nil, dependent_patient).parent_access?).to be(false)
      end
    end
  end

  describe ApplicationPolicy::Scope do
    it 'requires subclasses to implement #resolve' do
      expect { described_class.new(users(:admin), User.all).resolve }.to raise_error(NoMethodError, /resolve/)
    end

    describe 'patient-access helpers (via concrete subclass)' do
      # Concrete scope whose #resolve exposes the protected accessible_person_ids.
      let(:scope_class) do
        Class.new(ApplicationPolicy::Scope) do
          def resolve = accessible_person_ids
        end
      end

      def ids_for(user) = scope_class.new(user, Person.all).resolve

      it 'returns the carer person plus their active patient ids for a carer' do
        carer = users(:carer)
        expect(ids_for(carer)).to contain_exactly(
          carer.person_id,
          people(:child_patient).id,
          people(:child_user_person).id
        )
      end

      it 'excludes inactive carer relationships for a carer' do
        carer_relationships(:carer_cares_for_patient).deactivate!
        carer = users(:carer)
        # Only the remaining active relationship (child_user_person) plus self.
        expect(ids_for(carer)).to contain_exactly(
          carer.person_id,
          people(:child_user_person).id
        )
      end

      it 'returns the parent person plus their dependent patient ids for a parent' do
        parent = users(:parent)
        expect(ids_for(parent)).to contain_exactly(
          parent.person_id,
          people(:child_user_person).id
        )
      end

      it 'excludes dependent patients with capacity for a parent' do
        people(:child_user_person).update!(person_type: :adult, has_capacity: true)
        parent = users(:parent)
        expect(ids_for(parent)).to contain_exactly(parent.person_id)
      end

      it 'excludes a parent patient typed as a plain adult (person_type filter)' do
        people(:child_user_person).update!(person_type: :adult, has_capacity: false)
        parent = users(:parent)
        expect(ids_for(parent)).to contain_exactly(parent.person_id)
      end

      it 'excludes inactive relationships when collecting a parent dependent ids' do
        carer_relationships(:parent_cares_for_child).deactivate!
        parent = users(:parent)
        expect(ids_for(parent)).to contain_exactly(parent.person_id)
      end

      it 'returns only the user person for a clinician with no carer relationships' do
        doctor = users(:doctor)
        expect(ids_for(doctor)).to contain_exactly(doctor.person_id)
      end

      it 'returns an empty list when the user has no person' do
        user_without_person = User.new(role: :carer)
        expect(ids_for(user_without_person)).to eq([])
      end
    end
  end
end
