# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminDashboardPolicy, type: :policy do
  fixtures :all

  it 'permits index? only for administrators' do
    expect(described_class.new(users(:admin), :dashboard).index?).to be(true)
    expect(described_class.new(users(:doctor), :dashboard).index?).to be(false)
    expect(described_class.new(nil, :dashboard).index?).to be(false)
  end

  describe AdminDashboardPolicy::Scope do
    it 'returns the given scope unchanged' do
      scope = User.all
      expect(described_class.new(users(:admin), scope).resolve).to eq(scope)
    end
  end
end
