# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Controller concerns' do
  it 'does not keep single-use schedule index person resolution as a concern' do
    expect(Rails.root.join('app/controllers/concerns/schedule_index_person_resolvable.rb')).not_to exist
    expect(SchedulesController.ancestors.map(&:name)).not_to include('ScheduleIndexPersonResolvable')
  end
end
