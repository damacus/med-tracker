# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::ScheduleCard, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:person) { people(:john) }
  let(:schedule) { schedules(:active_schedule) }

  describe 'rendering' do
    it 'renders the person name' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(person.name)
    end

    it 'renders the medication name' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(schedule.medication.name)
    end

    it 'renders the schedule frequency' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(schedule.frequency)
    end

    it 'renders the medication quantity' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(schedule.medication.current_supply.to_s)
    end
  end

  describe 'card structure' do
    it 'renders with a schedule-specific id' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.css("#schedule_#{schedule.id}")).to be_present
    end

    it 'renders dosage details' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include('Dosage')
      expect(rendered.text).to include('Frequency')
    end

    it 'renders end date information' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include('Ends')
    end
  end
end
