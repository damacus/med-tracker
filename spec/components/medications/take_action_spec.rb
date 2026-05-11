# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::TakeAction, type: :component do
  fixtures :accounts, :locations, :medications, :people, :users

  let(:person) { people(:jane) }
  let(:user) { users(:admin) }
  let(:medication) { medications(:ibuprofen) }
  let(:source) do
    Schedule.create!(
      person: person,
      medication: medication,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days,
      dose_amount: medication.dosage_amount,
      dose_unit: medication.dosage_unit
    )
  end

  it 'renders a RubyUI location modal with a time field defaulting to now and capped 60 minutes ahead' do
    travel_to(Time.zone.local(2026, 4, 28, 14, 45)) do
      build_alternate_medication

      rendered = render_take_action
      timestamp_field = rendered.at_css("input[type='time'][name='medication_take[taken_at]']")

      expect(rendered.text).to include('Record dose')
      expect(rendered.at_css("form[action='#{take_path}']")).not_to be_nil
      expect(timestamp_field).not_to be_nil
      expect(timestamp_field['value']).to eq('14:45')
      expect(timestamp_field['max']).to eq('15:45')
    end
  end

  it 'clamps the time field max at 23:59 when the 60-minute window crosses midnight' do
    travel_to(Time.zone.local(2026, 4, 28, 23, 30)) do
      build_alternate_medication

      rendered = render_take_action
      timestamp_field = rendered.at_css("input[type='time'][name='medication_take[taken_at]']")

      expect(timestamp_field['value']).to eq('23:30')
      expect(timestamp_field['max']).to eq('23:59')
    end
  end

  it 'renders decorative SVG icons on enabled and confirmation buttons' do
    rendered = render_take_action(button: { icon: Components::Icons::HandPackage })

    icons = rendered.css('button svg.material-symbol-hand-package')
    expect(icons.count).to eq(2)
    expect(icons.all? { |icon| icon['aria-hidden'] == 'true' }).to be(true)
  end

  it 'renders a decorative SVG icon on disabled buttons' do
    rendered = render_take_action(
      button: { icon: Components::Icons::HandPackage },
      state: { disabled: true, label: 'Out of Stock', icon: Components::Icons::AlertCircle }
    )

    icon = rendered.at_css("button[data-testid='take-schedule-#{source.id}-disabled'] svg.lucide-alert-circle")
    expect(icon).to be_present
    expect(icon['aria-hidden']).to eq('true')
    expect(rendered.text).to include('Out of Stock')
  end

  def render_take_action(button: {}, state: {})
    html = view_context.render(
      described_class.new(
        source: source,
        context: { person: person, current_user: user },
        amount: source.dose_amount,
        button: {
          label: 'Give dose',
          variant: :filled,
          testid: "take-schedule-#{source.id}"
        }.merge(button),
        state: state
      )
    )
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def take_path
    Rails.application.routes.url_helpers.take_medication_person_schedule_path(person, source)
  end

  def build_alternate_medication
    Medication.create!(
      name: medication.name,
      location: locations(:school),
      category: medication.category,
      dosage_amount: medication.dosage_amount,
      dosage_unit: medication.dosage_unit,
      current_supply: 7,
      reorder_threshold: 1
    )
  end
end
