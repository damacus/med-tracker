# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe ReportPolicy, type: :policy do
  subject(:policy) { described_class.new(user, :report) }

  context 'with a household manager context' do
    let(:user) { household_policy_member(role: :owner).fetch(:context) }

    it { is_expected.to permit_action(:index) }
  end

  context 'with an adult household member context' do
    let(:user) { household_policy_member(role: :member).fetch(:context) }

    it { is_expected.to permit_action(:index) }
  end

  context 'with a revoked household membership context' do
    let(:member) { household_policy_member(role: :member) }
    let(:user) do
      member.fetch(:membership).update!(status: :revoked)
      member.fetch(:context)
    end

    it { is_expected.not_to permit_action(:index) }
  end

  context 'with a legacy user' do
    let(:user) { User.new }

    it { is_expected.not_to permit_action(:index) }
  end
end
