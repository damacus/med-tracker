# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe DashboardPolicy do
  fixtures :all

  subject(:policy) { described_class.new(user, :dashboard) }

  describe '#index?' do
    context 'when user is authenticated' do
      let(:user) { users(:admin) }

      it { is_expected.to permit_action(:index) }
    end

    context 'when user is a doctor' do
      let(:user) { users(:doctor) }

      it { is_expected.to permit_action(:index) }
    end

    context 'when user is a nurse' do
      let(:user) { users(:nurse) }

      it { is_expected.to permit_action(:index) }
    end

    context 'when user is a carer' do
      let(:user) { users(:carer) }

      it { is_expected.to permit_action(:index) }
    end

    context 'when user is a parent' do
      let(:user) { users(:parent) }

      it { is_expected.to permit_action(:index) }
    end

    context 'when user is an adult patient' do
      let(:user) { users(:adult_patient) }

      it { is_expected.to permit_action(:index) }
    end

    context 'when user is nil' do
      let(:user) { nil }

      it { is_expected.not_to permit_action(:index) }
    end
  end
end
