# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen') }
  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: 'mg',
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  describe 'i18n translations' do
    it 'renders card with default locale translations' do
      vc = view_context
      vc.singleton_class.define_method(:current_user) { nil }

      html = vc.render(described_class.new(schedule: schedule, person: person))
      rendered = Nokogiri::HTML::DocumentFragment.parse(html)
      text = rendered.text

      expect(text).to include("Today's Doses")
      expect(text).to include('No doses taken today')
      expect(text).to include('📅 Started:')
      expect(text).to include('🏁 Ends:')
    end

    it 'renders delete dialog with translated strings for admin user' do
      admin = instance_double(User, administrator?: true)
      vc = view_context
      vc.singleton_class.define_method(:current_user) { admin }

      html = vc.render(described_class.new(schedule: schedule, person: person))
      rendered = Nokogiri::HTML::DocumentFragment.parse(html)
      text = rendered.text

      expect(text).to include('Delete Schedule')
      expect(text).to include('Cancel')
    end
  end
end
