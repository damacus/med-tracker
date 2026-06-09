# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AppSettingsPolicy, type: :policy do
  fixtures :all

  it 'permits show? only for administrators' do
    expect(described_class.new(users(:admin), :settings).show?).to be(true)
    expect(described_class.new(users(:doctor), :settings).show?).to be(false)
    expect(described_class.new(nil, :settings).show?).to be_falsey
  end

  it 'permits update? only for administrators' do
    expect(described_class.new(users(:admin), :settings).update?).to be(true)
    expect(described_class.new(users(:nurse), :settings).update?).to be(false)
    expect(described_class.new(nil, :settings).update?).to be_falsey
  end

  describe AppSettingsPolicy::Scope do
    it 'returns the given scope unchanged' do
      scope = User.all
      expect(described_class.new(users(:admin), scope).resolve).to eq(scope)
    end
  end
end
