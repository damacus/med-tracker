# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::IndexView, type: :component do
  fixtures :people, :users, :medicines, :dosages, :prescriptions

  subject(:dashboard_view) do
    described_class.new(
      people: people,
      active_prescriptions: active_prescriptions,
      upcoming_prescriptions: upcoming_prescriptions
    )
  end

  let(:people) { Person.includes(:user, prescriptions: :medicine).all }
  let(:active_prescriptions) { Prescription.where(active: true).includes(person: :user, medicine: []) }
  let(:upcoming_prescriptions) { active_prescriptions.group_by(&:person) }

  it 'renders the dashboard title' do
    rendered = render_inline(dashboard_view)
    page_title = rendered.css('.page-title')
    expect(page_title.text).to eq('Medicine Tracker Dashboard')
  end

  describe 'stats display' do
    it 'renders people count' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include(people.count.to_s)
    end

    it 'renders active prescriptions count' do
      rendered = render_inline(dashboard_view)
      expect(rendered.text).to include(active_prescriptions.count.to_s)
    end
  end

  context 'when there are people but no prescriptions' do
    let(:active_prescriptions) { [] }
    let(:upcoming_prescriptions) { {} }

    it 'renders the empty prescriptions message' do
      rendered = render_inline(dashboard_view)
      empty_message = rendered.at_css('.empty-state__message')
      expect(empty_message).to be_present
      expect(empty_message.text).to include('No active prescriptions found')
    end
  end

  context 'when there are no people' do
    let(:people) { [] }
    let(:active_prescriptions) { [] }
    let(:upcoming_prescriptions) { {} }

    it 'renders the empty people message' do
      rendered = render_inline(dashboard_view)
      empty_message = rendered.at_css('.empty-state__message')
      expect(empty_message).to be_present
      expect(empty_message.text).to include('No people found')
    end
  end
end
