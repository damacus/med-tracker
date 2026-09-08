# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OfflineDoseEligibility do
  fixtures :accounts, :users, :people, :medications, :schedules

  it 'does not offer recording to a view-only user' do
    result = described_class.new(
      source: schedules(:john_paracetamol), user: users(:admin),
      policy: instance_double(SchedulePolicy, take_medication?: false)
    ).as_json

    expect(result).to include(allowed: false, reason: I18n.t('offline.record_access_required'))
  end
end
