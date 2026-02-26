# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  describe 'i18n translations' do
    it 'renders delete dialog with default locale translations' do
      medication = instance_double(Medication, name: 'Aspirin')
      person = instance_double(Person, name: 'John Doe')
      schedule = instance_double(Schedule, id: 1, medication: medication, person: person)

      component = described_class.new(
        schedule: schedule
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Delete')
      expect(rendered.to_html).to include('Delete Schedule?')
      expect(rendered.to_html).to include('Cancel')
    end
  end
end
