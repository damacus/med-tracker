# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApplicationPolicy, type: :policy do
  subject(:policy) { described_class.new(household_policy_member(role: :owner).fetch(:context), :record) }

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
      expect do
        described_class.new(household_policy_member(role: :member).fetch(:context),
                            :rec).send(:person_id_for_authorization)
      end
        .to raise_error(NotImplementedError, /person_id_for_authorization/)
    end
  end

  describe 'legacy relationship predicates' do
    let(:policy_class) do
      Class.new(ApplicationPolicy) do
        def carer_access? = carer_with_patient?
        def parent_access? = parent_with_dependent_patient?
      end
    end

    it 'does not authorize through old carer or parent relationships' do
      legacy_user = User.new(person: Person.new(name: 'Legacy', date_of_birth: 30.years.ago))
      policy = policy_class.new(legacy_user, Person.new)

      expect(policy.carer_access?).to be(false)
      expect(policy.parent_access?).to be(false)
    end
  end

  describe ApplicationPolicy::Scope do
    it 'requires subclasses to implement #resolve' do
      expect { described_class.new(household_policy_member(role: :owner).fetch(:context), User.all).resolve }
        .to raise_error(NoMethodError, /resolve/)
    end

    it 'does not derive accessible people from legacy user relationships' do
      scope_class = Class.new(ApplicationPolicy::Scope) do
        def resolve = accessible_person_ids
      end

      expect(scope_class.new(User.new, Person.all).resolve).to eq([])
    end
  end
end
