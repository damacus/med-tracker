# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::SelectableOption, type: :component do
  it 'renders a checked checkbox option card' do
    option = render_option(checked: true)

    aggregate_failures do
      expect(option.at_css('label')).to be_present
      expect(option.at_css('input[type="checkbox"]')['checked']).to eq('checked')
      expect(option.text).to include('Jane Doe')
      expect(option.text).to include('Dependent patient')
      expect(option.at_css('label')['class']).to include('has-[:checked]:border-primary')
    end
  end

  it 'renders an unchecked radio option card' do
    option = render_option(type: :radio, name: 'schedule_type', value: 'daily', checked: false)
    input = option.at_css('input[type="radio"]')

    aggregate_failures do
      expect(input['name']).to eq('schedule_type')
      expect(input['value']).to eq('daily')
      expect(input).not_to have_attribute('checked')
    end
  end

  it 'preserves disabled state and caller supplied data attributes' do
    option = render_option(
      disabled: true,
      data: { dependent_assignment_target: 'option', action: 'change->dependent-assignment#sync' }
    )
    input = option.at_css('input')

    aggregate_failures do
      expect(input).to have_attribute('disabled')
      expect(input['data-dependent-assignment-target']).to eq('option')
      expect(input['data-action']).to eq('change->dependent-assignment#sync')
      expect(option.at_css('label')['class']).to include('has-[:disabled]:cursor-not-allowed')
    end
  end

  it 'renders a hidden blank field when requested' do
    rendered = render_inline(
      described_class.new(
        type: :checkbox,
        name: 'user[dependent_ids][]',
        value: 1,
        input_id: 'user_dependent_1',
        label: 'Jane Doe',
        hidden_field: true
      )
    )

    hidden = rendered.at_css('input[type="hidden"][name="user[dependent_ids][]"]')

    expect(hidden['value']).to eq('')
  end

  def render_option(**attrs)
    render_inline(
      described_class.new(
        type: attrs.fetch(:type, :checkbox),
        name: attrs.fetch(:name, 'user[dependent_ids][]'),
        value: attrs.fetch(:value, 1),
        input_id: attrs.fetch(:input_id, 'user_dependent_1'),
        label: attrs.fetch(:label, 'Jane Doe'),
        description: attrs.fetch(:description, 'Dependent patient'),
        checked: attrs.fetch(:checked, false),
        disabled: attrs.fetch(:disabled, false),
        data: attrs.fetch(:data, {})
      )
    )
  end
end
