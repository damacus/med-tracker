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

  it 'shows the recorded location for today takes' do
    add_person_medication_take_for('PRN Alt Location')
    rendered = render_person_medication_card
    expect(rendered.text).to include('PRN Alt Location')
  end

  def render_person_medication_card
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?).new(false, true, false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(person_medication: person_medication, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def add_person_medication_take_for(location_name)
    alternate_location = create(:location, name: location_name)
    alternate_medication = create(
      :medication,
      name: medication.name,
      location: alternate_location,
      dosage_amount: medication.dosage_amount,
      dosage_unit: medication.dosage_unit
    )
    MedicationTake.create!(
      person_medication: person_medication,
      taken_at: Time.current,
      amount_ml: 1,
      taken_from_medication: alternate_medication,
      taken_from_location: alternate_location
    )
  end
end
