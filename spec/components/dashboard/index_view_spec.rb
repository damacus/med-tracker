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
    # Use Nokogiri methods instead of Capybara matchers
    page_title = rendered.css('.page-title')
    expect(page_title).to be_present
    expect(page_title.text.strip).to include('Medicine Tracker Dashboard')
  end

  describe 'stats display' do
    it 'renders users count' do
      rendered = render_inline(dashboard_view)
      # Use Nokogiri methods instead of Capybara matchers
      stat_numbers = rendered.css('.stat-card__number')
      expect(stat_numbers.any? { |node| node.text.strip == users.count.to_s }).to be true
    end

    it 'renders active prescriptions count' do
      rendered = render_inline(dashboard_view)
      # Use Nokogiri methods instead of Capybara matchers
      stat_numbers = rendered.css('.stat-card__number')
      expect(stat_numbers.any? { |node| node.text.strip == active_prescriptions.count.to_s }).to be true
    end
  end

  it 'renders quick actions' do
    rendered = render_inline(dashboard_view)
    # Use Nokogiri methods instead of Capybara matchers
    links = rendered.css('a')
    add_medicine_link = links.find { |link| link.text.strip == 'Add Medicine' }
    expect(add_medicine_link).to be_present
    expect(add_medicine_link['href']).to eq('#')
  end

  context 'when there are users and prescriptions' do
    it 'renders the medication schedule' do
      allow(upcoming_prescriptions).to receive(:any?).and_return(true)
      rendered = render_inline(dashboard_view)
      # Use Nokogiri methods instead of Capybara matchers
      schedule_content = rendered.css('.schedule-content')
      expect(schedule_content).to be_present
    end

    it 'does not render the empty state' do
      allow(upcoming_prescriptions).to receive(:any?).and_return(true)
      rendered = render_inline(dashboard_view)
      # Use Nokogiri methods instead of Capybara matchers
      empty_state = rendered.css('.empty-state')
      expect(empty_state).to be_empty
    end
  end

  context 'when there are users but no prescriptions' do
    it 'renders the empty prescriptions message' do
      allow(upcoming_prescriptions).to receive(:any?).and_return(false)
      rendered = render_inline(dashboard_view)
      # Use Nokogiri methods instead of Capybara matchers
      empty_message = rendered.css('.empty-state__message')
      expect(empty_message).to be_present
      expect(empty_message.text).to include('No active prescriptions found')
    end
  end

  context 'when there are no users' do
    it 'renders the empty users message' do
      allow(users).to receive(:any?).and_return(false)
      rendered = render_inline(dashboard_view)
      # Use Nokogiri methods instead of Capybara matchers
      empty_message = rendered.css('.empty-state__message')
      expect(empty_message).to be_present
      expect(empty_message.text).to include('No users found')
    end
  end
end
