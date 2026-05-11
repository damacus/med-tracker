# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::AdministrationModal, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:medication) { medications(:ibuprofen) }
  let(:schedule) { schedules(:jane_ibuprofen) }
  let(:current_user) { users(:admin) }

  it 'renders each administration action with a hand package icon' do
    rendered = render_inline(
      described_class.new(
        medication: medication,
        schedules: [schedule],
        person_medications: [],
        current_user: current_user
      )
    )

    selector = "button[data-testid='log-administration-schedule-#{schedule.id}'] svg.material-symbol-hand-package"

    expect(rendered.css(selector)).to be_present
  end
end
