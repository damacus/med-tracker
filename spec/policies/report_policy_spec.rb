# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe ReportPolicy do
  fixtures :all

  subject(:policy) { described_class.new(user, :report) }

  context 'when user is an administrator' do
    let(:user) { users(:admin) }

    it { is_expected.to permit_action(:index) }
  end

  context 'when user is a doctor' do
    let(:user) { users(:doctor) }

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

  context 'when user is a minor patient' do
    let(:user) { users(:minor_patient_user) }

    it { is_expected.to forbid_action(:index) }
  end
end
