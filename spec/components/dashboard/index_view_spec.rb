# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::IndexView, type: :component do
  fixtures :users, :medicines, :dosages, :prescriptions

  subject(:dashboard_view) do
    described_class.new(
      users: users,
      active_prescriptions: active_prescriptions,
      upcoming_prescriptions: upcoming_prescriptions
    )
  end

  let(:users) { User.includes(prescriptions: :medicine).all }
  let(:active_prescriptions) { Prescription.where(active: true).includes(:user, :medicine) }
  let(:upcoming_prescriptions) { active_prescriptions.group_by(&:user) }

  it 'renders the dashboard title' do
    rendered = render_inline(dashboard_view)
    expect(rendered).to have_css('.page-title', text: 'Medicine Tracker Dashboard')
  end

  describe 'stats display' do
    it 'renders users count' do
      rendered = render_inline(dashboard_view)
      expect(rendered).to have_css('.stat-card__number', text: users.count.to_s)
    end

    it 'renders active prescriptions count' do
      rendered = render_inline(dashboard_view)
      expect(rendered).to have_css('.stat-card__number', text: active_prescriptions.count.to_s)
    end
  end

  it 'renders quick actions' do
    rendered = render_inline(dashboard_view)
    expect(rendered).to have_link('Add Medicine', href: '#')
  end

  context 'when there are users and prescriptions' do
    it 'renders the medication schedule' do
      allow(upcoming_prescriptions).to receive(:any?).and_return(true)
      rendered = render_inline(dashboard_view)
      expect(rendered).to have_css('.schedule-content')
    end

    it 'does not render the empty state' do
      allow(upcoming_prescriptions).to receive(:any?).and_return(true)
      rendered = render_inline(dashboard_view)
      expect(rendered).not_to have_css('.empty-state')
    end
  end

  context 'when there are users but no prescriptions' do
    it 'renders the empty prescriptions message' do
      allow(upcoming_prescriptions).to receive(:any?).and_return(false)
      rendered = render_inline(dashboard_view)
      expect(rendered).to have_css('.empty-state__message', text: 'No active prescriptions found')
    end
  end

  context 'when there are no users' do
    it 'renders the empty users message' do
      allow(users).to receive(:any?).and_return(false)
      rendered = render_inline(dashboard_view)
      expect(rendered).to have_css('.empty-state__message', text: 'No users found')
    end
  end
end
