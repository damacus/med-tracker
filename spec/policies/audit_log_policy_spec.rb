# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditLogPolicy do
  subject(:policy) { described_class.new(user, :audit_log) }

  context 'when user is an administrator' do
    let(:user) { User.new(role: :administrator) }

    it { is_expected.to permit_action(:index) }
    it { is_expected.to permit_action(:show) }
    it { is_expected.to forbid_action(:create) }
    it { is_expected.to forbid_action(:update) }
    it { is_expected.to forbid_action(:destroy) }
  end

  context 'when user is a doctor' do
    let(:user) { User.new(role: :doctor) }

    it { is_expected.to forbid_action(:index) }
    it { is_expected.to forbid_action(:show) }
  end

  context 'when user is a nurse' do
    let(:user) { User.new(role: :nurse) }

    it { is_expected.to forbid_action(:index) }
    it { is_expected.to forbid_action(:show) }
  end

  context 'when user is a carer' do
    let(:user) { User.new(role: :carer) }

    it { is_expected.to forbid_action(:index) }
    it { is_expected.to forbid_action(:show) }
  end

  context 'when user is a parent' do
    let(:user) { User.new(role: :parent) }

    it { is_expected.to forbid_action(:index) }
    it { is_expected.to forbid_action(:show) }
  end
end
