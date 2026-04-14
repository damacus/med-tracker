# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::DashboardMetricsQuery do
  fixtures :accounts, :people, :users, :schedules, :medications, :locations, :location_memberships, :carer_relationships

  describe '#call' do
    it 'returns the current count metrics' do
      result = described_class.new.call

      expect(result[:total_users]).to eq(User.count)
      expect(result[:active_users]).to eq(User.active.count)
      expect(result[:recent_signups]).to eq(User.where(created_at: 7.days.ago..).count)
      expect(result[:total_people]).to eq(Person.count)
      expect(result[:active_schedules]).to eq(Schedule.where(active: true).count)
    end

    it 'returns the current grouped metrics' do
      result = described_class.new.call

      expect(result[:users_by_role]).to eq(User.group(:role).count)
      expect(result[:people_by_type]).to eq(Person.group(:person_type).count)
    end

    it 'counts people without capacity who have no active carers' do
      no_carer = create(:person)
      no_carer.has_capacity = false
      no_carer.save!(validate: false)

      inactive_only = create(:person)
      inactive_only.has_capacity = false
      inactive_only.save!(validate: false)
      create(:carer_relationship, patient: inactive_only, carer: people(:jane), active: false)

      active_carer = create(:person)
      active_carer.has_capacity = false
      active_carer.save!(validate: false)
      create(:carer_relationship, patient: active_carer, carer: people(:jane), active: true)

      result = described_class.new.call

      expect(result[:patients_without_carers]).to be >= 2
    end
  end
end
