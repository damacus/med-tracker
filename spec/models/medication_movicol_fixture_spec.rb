# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Medication do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :schedules

  let(:canonical_name) do
    'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets'
  end

  it 'keeps the long imported name while displaying the friendly name for John Doe' do
    movicol = medications(:movicol)
    schedule = schedules(:john_movicol)

    expect(movicol.name).to eq(canonical_name)
    expect(movicol.display_name).to eq('Movicol Paediatric Plain')
    expect(schedule.person).to eq(people(:john))
    expect(schedule.medication).to eq(movicol)
  end
end
