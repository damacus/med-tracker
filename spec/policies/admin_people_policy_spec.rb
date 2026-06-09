# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminPeoplePolicy do
  fixtures :accounts, :people, :users

  subject(:policy) { described_class.new(user, :admin_people) }

  context 'with an administrator' do
    let(:user) { users(:admin) }

    it { expect(policy.index?).to be(true) }
  end

  context 'with a non-administrator' do
    let(:user) { users(:jane) }

    it { expect(policy.index?).to be(false) }
  end

  context 'with no user' do
    let(:user) { nil }

    it { expect(policy.index?).to be(false) }
  end
end
