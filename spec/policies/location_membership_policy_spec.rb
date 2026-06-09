# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LocationMembershipPolicy, type: :policy do
  fixtures :all

  it 'permits create? only for administrators' do
    expect(described_class.new(users(:admin), :membership).create?).to be(true)
    expect(described_class.new(users(:doctor), :membership).create?).to be(false)
    expect(described_class.new(nil, :membership).create?).to be(false)
  end

  it 'permits destroy? only for administrators' do
    expect(described_class.new(users(:admin), :membership).destroy?).to be(true)
    expect(described_class.new(users(:nurse), :membership).destroy?).to be(false)
    expect(described_class.new(nil, :membership).destroy?).to be(false)
  end
end
