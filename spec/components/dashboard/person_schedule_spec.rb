# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PersonSchedule, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  subject(:component) { described_class.new(person: person, schedules: schedules) }

  let(:person) { people(:john) }
  let(:schedules) { person.schedules.where(active: true) }

  before do
    MedicationTake.where(schedule: schedules).delete_all
  end

  it 'renders the person\'s name' do
    rendered = render_inline(component)
    expect(rendered.text).to include(person.name)
  end

  it 'renders each schedule' do
    rendered = render_inline(component)

    schedules.each do |schedule|
      schedule_element = rendered.css("#schedule_#{schedule.id}")
      expect(schedule_element).to be_present
      expect(rendered.text).to include(schedule.medication.name)
    end
  end

  it 'renders take now links for each schedule' do
    rendered = render_inline(component)

    schedules.each do |schedule|
      link = rendered.css("[data-test-id='take-medication-#{schedule.id}']")
      expect(link).to be_present
      expect(link.text).to include('Take Now')
    end
  end
end
