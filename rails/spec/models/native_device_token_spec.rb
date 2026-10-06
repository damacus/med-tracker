# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NativeDeviceToken do
  fixtures :accounts, :people, :users

  subject { described_class.new(platform: 'ios', account: accounts(:admin)) }

  it { is_expected.to belong_to(:account) }
  it { is_expected.to validate_presence_of(:device_token) }
  it { is_expected.to validate_inclusion_of(:platform).in_array(%w[ios android]) }

  it 'enforces uniqueness of device_token' do
    described_class.create!(account: accounts(:admin), device_token: 'unique-tok', platform: 'ios')
    duplicate = described_class.new(account: accounts(:admin), device_token: 'unique-tok', platform: 'android')
    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:device_token]).to be_present
  end

  it 'freezes the platform allow-list' do
    expect(described_class::PLATFORMS).to eq(%w[ios android]).and be_frozen
  end
end
