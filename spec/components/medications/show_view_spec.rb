# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::ShowView, type: :component do
  let(:medication) { create(:medication, name: 'Paracetamol', current_supply: 50) }

  def fetch_action_element(rendered, selector, message)
    rendered.at_css(selector) || raise(message)
  end

  it 'renders the medication name' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'renders action links using Link component without raw button classes' do
    rendered = render_inline(described_class.new(medication: medication))

    edit_link = rendered.css('a').find { |a| a.text.include?('Edit Details') }
    back_link = rendered.css('a').find { |a| a.text.include?('Inventory') }
    expect(edit_link).to be_present
    expect(back_link).to be_present
  end

  it 'renders the inventory status heading' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Inventory Status')
  end

  it 'offsets the content to align with the header title column' do
    rendered = render_inline(described_class.new(medication: medication))

    content = rendered.at_css("[data-testid='medication-content']")

    expect(content).to be_present
    expect(content[:class]).to include('md:pl-[6.5rem]')
  end

  describe 'action button styling' do
    let(:medication) { create(:medication, name: 'Paracetamol', current_supply: 50, reorder_status: nil) }
    let(:rendered) { render_inline(described_class.new(medication: medication)) }

    it 'uses action button utility tokens for add schedule' do
      add_schedule_link = fetch_action_element(rendered, "a[href*='/add_medication']", 'Add Schedule link not found')

      expect(add_schedule_link[:class]).to include('rounded-shape-full')
    end

    it 'uses action button utility tokens for log administration' do
      log_administration_link = fetch_action_element(
        rendered,
        "a[href$='/administration']",
        'Log Administration link not found'
      )

      expect(log_administration_link[:class]).to include('rounded-shape-full')
    end

    it 'uses action button utility tokens for mark as ordered' do
      mark_as_ordered_link = fetch_action_element(
        rendered,
        "a[href$='/mark_as_ordered']",
        'Mark as Ordered link not found'
      )

      expect(mark_as_ordered_link[:class]).to include('rounded-shape-full')
    end

    it 'uses action button utility tokens for refill' do
      refill_button = fetch_action_element(
        rendered,
        "[data-controller='ruby-ui--dialog'] button",
        'Refill button not found'
      )

      expect(refill_button[:class]).to include('rounded-shape-full')
    end
  end

  it 'renders the log administration link' do
    rendered = render_inline(described_class.new(medication: medication))

    log_administration_link = rendered.css('a').find { |link| link.text.include?('Log') }

    expect(log_administration_link).to be_present
  end

  it 'renders the log administration href' do
    rendered = render_inline(described_class.new(medication: medication))

    log_administration_link = rendered.css('a').find { |link| link.text.include?('Log') }

    expect(log_administration_link[:href]).to eq("/medications/#{medication.id}/administration")
  end

  it 'renders log administration as a modal link' do
    rendered = render_inline(described_class.new(medication: medication))

    log_administration_link = rendered.css('a').find { |link| link.text.include?('Log') }

    expect(log_administration_link['data-turbo-frame']).to eq('modal')
  end

  it 'renders safety warnings when present' do
    medication_with_warnings = create(:medication, warnings: 'Take with food')
    rendered = render_inline(described_class.new(medication: medication_with_warnings))

    expect(rendered.text).to include('Safety Warnings')
  end

  it 'renders warning content when present' do
    medication_with_warnings = create(:medication, warnings: 'Take with food')
    rendered = render_inline(described_class.new(medication: medication_with_warnings))

    expect(rendered.text).to include('Take with food')
  end

  context 'when forecast is available' do
    it 'renders the out-of-stock forecast' do
      medication_with_schedule = create(:medication, name: 'Paracetamol', current_supply: 50)
      create(
        :schedule,
        medication: medication_with_schedule,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 10,
        dose_cycle: :daily
      )

      rendered = render_inline(described_class.new(medication: medication_with_schedule))

      expect(rendered.text).to include('Supply will be empty in 5 days')
    end

    it 'renders the low-stock forecast' do
      medication_with_schedule = create(:medication, name: 'Paracetamol', current_supply: 50)
      create(
        :schedule,
        medication: medication_with_schedule,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 10,
        dose_cycle: :daily
      )

      rendered = render_inline(described_class.new(medication: medication_with_schedule))

      # With current_supply: 50, reorder_threshold defaults to 10 from migration
      # Surplus = 50 - 10 = 40, days = (40 / 10).ceil = 4
      expect(rendered.text).to include('Supply will be low in 4 days')
    end
  end

  context 'when forecast is not available' do
    it 'renders the fallback message' do
      rendered = render_inline(described_class.new(medication: medication))

      expect(rendered.text).to include('Forecast unavailable')
    end
  end
end
