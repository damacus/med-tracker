# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PrescriptionCard, type: :component do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  let(:person) { people(:john) }
  let(:prescription) { prescriptions(:active_prescription) }

  describe 'rendering' do
    it 'renders the person name' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.text).to include(person.name)
    end

    it 'renders the medicine name' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.text).to include(prescription.medicine.name)
    end

    it 'renders the prescription frequency' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.text).to include(prescription.frequency)
    end

    it 'renders the medicine quantity' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.text).to include(prescription.medicine.current_supply.to_s)
    end
  end

  describe 'card structure' do
    it 'renders with a prescription-specific id' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.css("#prescription_#{prescription.id}")).to be_present
    end

    it 'renders dosage details' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.text).to include('Dosage')
      expect(rendered.text).to include('Frequency')
    end

    it 'renders end date information' do
      rendered = render_inline(described_class.new(person: person, prescription: prescription))

      expect(rendered.text).to include('Ends')
    end
  end
end
