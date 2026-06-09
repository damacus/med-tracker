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

  describe '#call attention and operational keys' do
    before do
      Invitation.delete_all
      NhsDmdImport.delete_all
      PaperTrail::Version.delete_all
      Person.find_each { |person| person.update!(has_capacity: true) }
    end

    it 'returns pending and expired invitation counts' do
      create(:invitation, expires_at: 2.days.from_now)
      create(:invitation, expires_at: 1.day.ago)

      result = described_class.new.call

      expect(result[:pending_invitations]).to eq(Invitation.pending.count)
      expect(result[:expired_invitations]).to eq(Invitation.expired.count)
    end

    it 'returns a 24-hour audit event count' do
      PaperTrail.request.whodunnit = users(:admin).id
      PaperTrail.request(enabled: true) { people(:john).update!(name: 'Changed') }

      expect(described_class.new.call[:recent_audit_events]).to be >= 1
    end

    it 'returns the latest three audit versions, newest first' do
      PaperTrail.request.whodunnit = users(:admin).id
      PaperTrail.request(enabled: true) do
        4.times { |index| people(:john).update!(name: "Name #{index}") }
      end

      activity = described_class.new.call[:recent_activity]

      expect(activity.size).to eq(3)
      expect(activity).to eq(activity.sort_by(&:created_at).reverse)
    end

    it 'builds a high carer attention item linking to admin people' do
      patient = create(:person)
      patient.has_capacity = false
      patient.save!(validate: false)

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:href] == '/admin/people'
      end

      expect(item).to include(
        severity: :high,
        action_label: I18n.t('admin.dashboard.attention.actions.view')
      )
    end

    it 'builds a medium expired-invitations item linking to admin invitations' do
      create(:invitation, expires_at: 1.day.ago)

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:href] == '/admin/invitations'
      end

      expect(item).to include(severity: :medium)
    end

    it 'does not add an item for normal pending invitations' do
      create(:invitation, expires_at: 2.days.from_now)

      hrefs = described_class.new.call[:attention_items].pluck(:href)

      expect(hrefs).not_to include('/admin/invitations')
    end

    it 'escalates a failed dm+d import to high severity' do
      NhsDmdImport.create!(uploaded_filename: 'release.zip', status: :failed, completed_at: 3.hours.ago)

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:icon_type] == 'refresh_cw'
      end

      expect(item).to include(severity: :high)
    end

    it 'escalates a long-running active dm+d import to medium severity' do
      NhsDmdImport.create!(
        uploaded_filename: 'release.zip',
        status: :importing,
        started_at: 2.hours.ago,
        created_at: 2.hours.ago
      )

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:icon_type] == 'refresh_cw'
      end

      expect(item).to include(severity: :medium)
    end

    it 'does not add an item for a completed dm+d import' do
      NhsDmdImport.create!(uploaded_filename: 'release.zip', status: :completed, completed_at: 1.hour.ago)

      icons = described_class.new.call[:attention_items].pluck(:icon_type)

      expect(icons).not_to include('refresh_cw')
    end

    it 'returns an empty attention list when nothing is actionable' do
      expect(described_class.new.call[:attention_items]).to eq([])
    end
  end
end
