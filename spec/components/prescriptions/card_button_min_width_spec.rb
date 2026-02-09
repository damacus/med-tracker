# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Prescriptions::Card, type: :component do
  describe 'Take button min-width' do
    it 'has min-w-[80px] for visual stability in Prescriptions::Card' do
      source = Rails.root.join('app/components/prescriptions/card.rb').read
      take_button_section = source[/Button\(\s*type: :submit.*?end/m]
      expect(take_button_section).to include('min-w-[80px]'),
                                     'Prescriptions Take button should have min-w-[80px] for visual stability'
    end

    it 'has min-w-[80px] for visual stability in PersonMedicines::Card' do
      source = Rails.root.join('app/components/person_medicines/card.rb').read
      take_button_section = source[/Button\(\s*type: :submit.*?end/m]
      expect(take_button_section).to include('min-w-[80px]'),
                                     'PersonMedicines Take button should have min-w-[80px] for visual stability'
    end
  end
end
