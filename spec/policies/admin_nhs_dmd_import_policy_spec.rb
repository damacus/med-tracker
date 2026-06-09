# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminNhsDmdImportPolicy, type: :policy do
  fixtures :all

  it 'permits new? only for administrators' do
    expect(described_class.new(users(:admin), :import).new?).to be(true)
    expect(described_class.new(users(:doctor), :import).new?).to be(false)
    expect(described_class.new(nil, :import).new?).to be_nil
  end

  it 'permits create? only for administrators' do
    expect(described_class.new(users(:admin), :import).create?).to be(true)
    expect(described_class.new(users(:nurse), :import).create?).to be(false)
    expect(described_class.new(nil, :import).create?).to be_nil
  end
end
