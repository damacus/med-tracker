# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::Wizard::StepDoseSchedule, type: :component do
  let(:person) { create(:person, name: 'Harrison Webb') }
  let(:medication) { create(:medication) }

  before do
    create(:dosage, medication:, amount: 2, unit: 'gummy')
  end

  it 'renders rounded RubyUI-style form controls for dose setup' do
    rendered = render_inline(described_class.new(medication:, people: [person]))

    expect(rendered.at_css('select#wizard_schedule_person')['class']).to include('rounded-shape-sm')
    expect(rendered.at_css('input#wizard_dose_amount')['class']).to include('rounded-shape-sm')
    expect(rendered.at_css('select#wizard_dose_unit')['class']).to include('rounded-shape-sm')
  end
end
