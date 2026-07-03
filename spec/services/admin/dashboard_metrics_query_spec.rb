# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::DashboardMetricsQuery do
  fixtures :accounts, :people, :users, :schedules, :medications, :locations, :location_memberships, :carer_relationships

  let(:household) { users(:admin).person.household }

  around do |example|
    Current.household = household
    attach_metrics_household_users
    example.run
  ensure
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
    Current.reset
  end

  describe '#call' do
    it 'returns the current count metrics' do
      result = described_class.new.call

      household_users = User.joins(person: { account: :household_memberships })
                            .where(household_memberships: { household: household })
                            .distinct
      expect(result[:total_users]).to eq(household_users.count)
      expect(result[:active_users]).to eq(household_users.active.count)
      expect(result[:recent_signups]).to eq(household_users.where(created_at: 7.days.ago..).count)
      expect(result[:total_people]).to eq(household.people.count)
      expect(result[:active_schedules]).to eq(household.schedules.where(active: true).count)
    end

    it 'returns the current grouped metrics' do
      result = described_class.new.call

      expect(result[:users_by_role]).to eq(household.household_memberships.active.group(:role).count)
      expect(result[:people_by_type]).to eq(household.people.group(:person_type).count)
    end

    it 'counts people without capacity who have no active carers' do
      no_carer = create(:person, household: household)
      no_carer.has_capacity = false
      no_carer.save!(validate: false)

      inactive_only = create(:person, household: household)
      inactive_only.has_capacity = false
      inactive_only.save!(validate: false)
      create(:carer_relationship, patient: inactive_only, carer: people(:jane), active: false)

      active_carer = create(:person, household: household)
      active_carer.has_capacity = false
      active_carer.save!(validate: false)
      create(:carer_relationship, patient: active_carer, carer: people(:jane), active: true)

      result = described_class.new.call

      expect(result[:patients_without_carers]).to be >= 2
    end
  end

  describe '#call attention and operational keys' do
    before do
      HouseholdInvitation.delete_all
      NhsDmdImport.delete_all
      PaperTrail::Version.delete_all
      Person.find_each { |person| person.update!(has_capacity: true) }
    end

    it 'returns pending and expired invitation counts' do
      create_metrics_invitation(expires_at: 2.days.from_now)
      create_metrics_invitation(expires_at: 1.day.ago)

      result = described_class.new.call

      expect(result[:pending_invitations]).to eq(household.household_invitations.pending.count)
      expect(result[:expired_invitations]).to eq(household.household_invitations.expired.count)
    end

    it 'returns a 24-hour audit event count' do
      PaperTrail.request.whodunnit = users(:admin).id
      PaperTrail.request.controller_info = paper_trail_info
      PaperTrail.request(enabled: true) { people(:john).update!(name: 'Changed') }

      expect(described_class.new.call[:recent_audit_events]).to be >= 1
    end

    it 'returns the latest three audit versions, newest first' do
      PaperTrail.request.whodunnit = users(:admin).id
      PaperTrail.request.controller_info = paper_trail_info
      PaperTrail.request(enabled: true) do
        4.times { |index| people(:john).update!(name: "Name #{index}") }
      end

      activity = described_class.new.call[:recent_activity]

      expect(activity.size).to eq(3)
      expect(activity).to eq(activity.sort_by(&:created_at).reverse)
    end

    it 'builds a high carer attention item linking to admin people' do
      patient = create(:person, household: household)
      patient.has_capacity = false
      patient.save!(validate: false)

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:href] == Rails.application.routes.url_helpers.admin_people_path(household_slug: household.slug)
      end

      expect(item).to include(
        severity: :high,
        action_label: I18n.t('admin.dashboard.attention.actions.view')
      )
    end

    it 'builds a medium expired-invitations item linking to admin invitations' do
      create_metrics_invitation(expires_at: 1.day.ago)

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:href] ==
          Rails.application.routes.url_helpers.admin_invitations_path(household_slug: household.slug)
      end

      expect(item).to include(severity: :medium)
    end

    it 'does not add an item for normal pending invitations' do
      create_metrics_invitation(expires_at: 2.days.from_now)

      hrefs = described_class.new.call[:attention_items].pluck(:href)

      expect(hrefs).not_to include(
        Rails.application.routes.url_helpers.admin_invitations_path(household_slug: household.slug)
      )
    end

    it 'adds a high dm+d item when there has never been a release import' do
      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:icon_type] == 'refresh_cw'
      end

      expect(item).to include(
        severity: :high,
        title: I18n.t('admin.dashboard.attention.dmd_missing.title')
      )
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

    it 'does not add an item for a recent active dm+d import' do
      NhsDmdImport.create!(
        uploaded_filename: 'release.zip',
        status: :importing,
        started_at: 5.minutes.ago,
        created_at: 5.minutes.ago
      )

      icons = described_class.new.call[:attention_items].pluck(:icon_type)

      expect(icons).not_to include('refresh_cw')
    end

    it 'adds a medium dm+d item when the latest completed import is stale' do
      NhsDmdImport.create!(uploaded_filename: 'release.zip', status: :completed, completed_at: 40.days.ago)

      item = described_class.new.call[:attention_items].find do |attention_item|
        attention_item[:icon_type] == 'refresh_cw'
      end

      expect(item).to include(
        severity: :medium,
        title: I18n.t('admin.dashboard.attention.dmd_stale.title')
      )
    end

    it 'does not add an item for a recent completed dm+d import' do
      NhsDmdImport.create!(uploaded_filename: 'release.zip', status: :completed, completed_at: 1.hour.ago)

      icons = described_class.new.call[:attention_items].pluck(:icon_type)

      expect(icons).not_to include('refresh_cw')
    end

    it 'returns an empty attention list when nothing is actionable' do
      NhsDmdImport.create!(uploaded_filename: 'release.zip', status: :completed, completed_at: 1.hour.ago)

      expect(described_class.new.call[:attention_items]).to eq([])
    end

    it 'memoizes the latest dm+d import for a query instance' do
      NhsDmdImport.create!(uploaded_filename: 'release.zip', status: :completed, completed_at: 1.hour.ago)
      query = described_class.new

      query.call
      NhsDmdImport.create!(uploaded_filename: 'newer-release.zip', status: :failed, completed_at: Time.current)

      expect(query.call[:attention_items]).to eq([])
    end
  end

  def attach_metrics_household_users
    attach_metrics_user(users(:admin), :owner)
    attach_metrics_user(users(:jane), :member)
  end

  def attach_metrics_user(user, role)
    membership = household.household_memberships.find_or_create_by!(account: user.person.account) do |membership|
      membership.person = user.person
    end
    membership.update!(person: user.person, role: role, status: :active)
  end

  def create_metrics_invitation(expires_at:)
    create(
      :household_invitation,
      household: household,
      invited_by_membership: metrics_owner_membership,
      expires_at: expires_at
    )
  end

  def paper_trail_info
    {
      household_id: household.id,
      actor_membership_id: metrics_owner_membership.id
    }
  end

  def metrics_owner_membership
    household.household_memberships.owner.active.order(:id).first!
  end
end
