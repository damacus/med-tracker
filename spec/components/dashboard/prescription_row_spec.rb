# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PrescriptionRow, type: :component do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  subject(:row) do
    described_class.new(
      person: person,
      prescription: prescription
    )
  end

  let(:person) { people(:john) }
  let(:prescription) { prescriptions(:active_prescription) }

  it 'renders the medicine quantity' do
    rendered = render_inline(row)
    expect(rendered.text).to include(prescription.medicine.stock.to_s)
  end
end
