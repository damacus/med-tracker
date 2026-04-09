# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Paracetamol') }

  let(:person_medication) do
    PersonMedication.create!(
      person: person,
      medication: medication,
      notes: 'Take with food',
      max_daily_doses: 4,
      min_hours_between_doses: 4
    )
  end

  def render_card(take_medication: false)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    policy_stub = Struct.new(:update?, :take_medication?, :destroy?).new(false, take_medication, false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }

    html = vc.render(described_class.new(person_medication: person_medication, person: person))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  describe 'countdown notice styling' do
    before do
      MedicationTake.create!(person_medication: person_medication, taken_at: 1.hour.ago, amount_ml: 10.0)
      allow(person_medication).to receive_messages(can_take_now?: false, countdown_display: '3 hours')
    end

    it 'uses amber/warning styling distinct from notes' do
      rendered = render_card

      countdown_div = rendered.css('[class*="bg-warning-container"]').detect { |el| el.text.include?('Next dose') }
      notes_div = rendered.css('[class*="bg-primary-container"]').detect { |el| el.text.include?('Notes') }

      expect(countdown_div).to be_present, 'countdown notice should use warning-container background'
      expect(notes_div).to be_present, 'notes should use primary-container background'
    end

    it 'memoizes blocked reason resolution when the action button is visible' do
      resolver = instance_double(MedicationStockSourceResolver, blocked_reason: nil,
                                                                available_medications: [medication])
      allow(MedicationStockSourceResolver).to receive(:new).and_return(resolver)

      render_card(take_medication: true)

      expect(resolver).to have_received(:blocked_reason).once
    end
  end
end
