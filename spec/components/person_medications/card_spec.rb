# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, dosage_amount: 1, dosage_unit: 'tablet') }
  let(:person_medication) do
    create(:person_medication, person: person, medication: medication, max_daily_doses: nil,
                               min_hours_between_doses: nil)
  end

  it 'disables the take button when medication dose is invalid' do
    person_medication.dose_amount = 0
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?).new(false, true, false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }

    html = vc.render(described_class.new(person_medication: person_medication, person: person))
    rendered = Nokogiri::HTML::DocumentFragment.parse(html)

    button = rendered.at_css("button[data-testid='take-person-medication-#{person_medication.id}-disabled'][disabled]")
    expect(button).not_to be_nil
    expect(button.text).to include('Invalid Dose Configured')
  end
end
