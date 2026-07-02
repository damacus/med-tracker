# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::ShowView, type: :component do
  let(:medication) { create(:medication, name: 'Paracetamol', current_supply: 50) }

  before do
    policy_stub = Struct.new(:update?, :refill?, :destroy?).new(true, true, true)
    view_context.singleton_class.define_method(:policy) { |_record| policy_stub }
  end

  def fetch_action_element(rendered, selector, message)
    rendered.at_css(selector) || raise(message)
  end

  it 'renders the medication name' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'renders the friendly display name when present' do
    medication.update!(
      name: 'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets',
      friendly_name: 'Movicol Paediatric Plain'
    )

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.at_css('h1').text).to include('Movicol Paediatric Plain')
    expect(rendered.at_css('h1').text).not_to include('Norgine Pharmaceuticals')
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
      mark_as_ordered_button = fetch_action_element(
        rendered,
        "form[action$='/mark_as_ordered'] button",
        'Mark as Ordered button not found'
      )

      expect(mark_as_ordered_button[:class]).to include('rounded-shape-full')
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

    expect(log_administration_link[:href])
      .to eq("/households/test-household/medications/#{medication.id}/administration")
  end

  it 'renders log administration as a modal link' do
    rendered = render_inline(described_class.new(medication: medication))

    log_administration_link = rendered.css('a').find { |link| link.text.include?('Log') }

    expect(log_administration_link['data-turbo-frame']).to eq('modal')
  end

  it 'renders restock but hides update-only actions for restock-only access' do
    rendered = render_with_policy(Struct.new(:update?, :refill?, :destroy?).new(false, true, false))

    expect(rendered.text).to include('Restock')
    expect(rendered.text).not_to include('Edit Details')
    expect(rendered.text).not_to include('Adjust Inventory')
  end

  it 'renders order workflow fields before a medication is ordered', :aggregate_failures do
    rendered = render_inline(described_class.new(medication: medication))
    order_form = rendered.at_css("form[action$='/mark_as_ordered']")

    expect(order_form).to be_present
    expect(order_form['data-turbo']).to eq('false')
    expect(order_form.at_css("button[type='submit']")).to be_present
    expect(rendered.text).to include('Supplier')
    expect(rendered.text).to include('Quantity')
    expect(rendered.text).to include('Expected arrival')
  end

  it 'renders captured order details while waiting for delivery' do
    medication.update!(
      reorder_status: :ordered,
      ordered_at: Time.zone.local(2026, 5, 5, 9, 30),
      order_supplier: 'Boots',
      order_quantity: 2,
      expected_arrival_on: Date.new(2026, 5, 8)
    )

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Boots')
    expect(rendered.text).to include('2')
    expect(rendered.text).to include(I18n.l(Date.new(2026, 5, 8), format: :long))
    expect(rendered.css("a[href$='/mark_as_received']")).to be_present
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

  def render_with_policy(policy_stub)
    vc = view_context
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(medication: medication))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
