# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::Schedule, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:person) { people(:john) }
  let(:schedules_for_person) { person.schedules.where(active: true) }
  let(:upcoming_schedules) { { person => schedules_for_person } }

  it 'renders person schedules without passing a take medication URL generator' do
    allow(Components::Dashboard::PersonSchedule).to receive(:new).and_call_original

    render_inline(described_class.new(people: [person], upcoming_schedules: upcoming_schedules))

    expect(Components::Dashboard::PersonSchedule).to have_received(:new).with(
      person: person,
      schedules: schedules_for_person,
      current_user: nil
    )
  end
end
