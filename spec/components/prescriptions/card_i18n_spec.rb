# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Prescriptions::Card, type: :component do
  let(:person) { create(:person) }
  let(:medicine) { create(:medicine, name: 'Ibuprofen') }
  let(:dosage) { Dosage.create!(medicine: medicine, amount: 400.0, unit: 'mg', frequency: 'Twice daily') }
  let(:prescription) do
    Prescription.create!(
      person: person,
      medicine: medicine,
      dosage: dosage,
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  describe 'i18n translations' do
    it 'renders card with default locale translations' do
      vc = view_context
      vc.singleton_class.define_method(:current_user) { nil }

      html = vc.render(described_class.new(prescription: prescription, person: person))
      rendered = Nokogiri::HTML::DocumentFragment.parse(html)
      text = rendered.text

      expect(text).to include("Today's Doses")
      expect(text).to include('No doses taken today')
      expect(text).to include('📅 Started:')
      expect(text).to include('🏁 Ends:')
      expect(text).to include('💊 Take')
    end

    it 'renders delete dialog with translated strings for admin user' do
      admin = instance_double(User, administrator?: true)
      vc = view_context
      vc.singleton_class.define_method(:current_user) { admin }

      html = vc.render(described_class.new(prescription: prescription, person: person))
      rendered = Nokogiri::HTML::DocumentFragment.parse(html)
      text = rendered.text

      expect(text).to include('Delete Prescription')
      expect(text).to include('Cancel')
    end
  end
end
