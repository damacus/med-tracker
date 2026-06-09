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

  describe ApplicationPolicy::Scope do
    it 'requires subclasses to implement #resolve' do
      expect { described_class.new(users(:admin), User.all).resolve }.to raise_error(NoMethodError, /resolve/)
    end
  end
end
