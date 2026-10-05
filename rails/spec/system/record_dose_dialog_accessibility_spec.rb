# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Record dose dialog accessibility', :browser do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages

  let(:admin) { users(:admin) }
  let!(:schedule) do
    Schedule.create!(
      person: people(:jane),
      medication: medications(:ibuprofen),
      dosage: dosages(:ibuprofen_adult),
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
  end

  before do
    driven_by(:playwright)
    sign_in(admin)
  end

  [[1400, 1000], [390, 844]].each do |width, height|
    it "dismisses Record dose with Escape and restores focus at #{width}x#{height}" do
      page.current_window.resize_to(width, height)
      visit dashboard_path(dashboard_person_id: schedule.person.id)

      trigger = find("[data-testid='take-dose-schedule_#{schedule.id}']")
      trigger.click

      dialog = find('dialog[open][role="dialog"]', text: 'Record dose')
      expect(page.evaluate_script('arguments[0].matches(":modal")', dialog)).to be(true)
      expect(page).to have_css('body.overflow-hidden')
      expect(dialog['aria-modal']).to eq('true')
      expect(dialog['aria-labelledby']).to be_present
      expect(dialog['aria-describedby']).to be_present
      expect(dialog).to have_css("##{dialog['aria-labelledby']}", text: 'Record dose')
      expect(dialog).to have_css("##{dialog['aria-describedby']}")

      find("input[name='medication_take[taken_at]'][type='time']").click
      find('body').send_keys(:escape)

      expect(page).to have_no_css('[role="dialog"]')
      expect(page).to have_no_css('body.overflow-hidden')
      expect(page.evaluate_script('document.activeElement.dataset.testid')).to eq("take-dose-schedule_#{schedule.id}")
    end
  end

  it 'keeps keyboard focus in the record dose dialog while it is modal' do
    visit dashboard_path(dashboard_person_id: schedule.person.id)

    trigger = find("[data-testid='take-dose-schedule_#{schedule.id}']")
    trigger.click

    dialog = find('dialog[open][role="dialog"]', text: 'Record dose')
    tabbable_count = dialog.all(
      'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), ' \
      'select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
    ).count

    expect(tabbable_count).to be_positive
    expect(page).to have_css('[role="dialog"] :focus')
    expect(page.evaluate_script("document.activeElement.closest('[role=dialog]') !== null")).to be(true)

    (tabbable_count + 1).times do
      page.active_element.send_keys(:tab)
      active_element_in_dialog = page.evaluate_script(
        "document.activeElement.closest('[role=dialog]') === arguments[0]", dialog
      )
      expect(active_element_in_dialog).to be(true)
    end

    (tabbable_count + 1).times do
      page.active_element.send_keys(%i[shift tab])
      active_element_in_dialog = page.evaluate_script(
        "document.activeElement.closest('[role=dialog]') === arguments[0]", dialog
      )
      expect(active_element_in_dialog).to be(true)
    end
  end

  it 'dismisses Record dose when the native backdrop is clicked' do
    visit dashboard_path(dashboard_person_id: schedule.person.id)

    trigger = find("[data-testid='take-dose-schedule_#{schedule.id}']")
    trigger.click
    expect(page).to have_css('dialog[open][role="dialog"]', text: 'Record dose')

    page.driver.with_playwright_page { |playwright_page| playwright_page.mouse.click(5, 5) }

    expect(page).to have_no_css('[role="dialog"]')
    expect(page.evaluate_script('document.activeElement.dataset.testid')).to eq("take-dose-schedule_#{schedule.id}")
  end

  it 'uses unique accessible references for separate dialog instances' do
    second_schedule = Schedule.create!(
      person: schedule.person,
      medication: schedule.medication,
      dosage: schedule.dosage,
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
    visit dashboard_path(dashboard_person_id: schedule.person.id)

    find("[data-testid='take-dose-schedule_#{schedule.id}']").click
    first_dialog = find('dialog[open][role="dialog"]', text: 'Record dose')
    first_references = [first_dialog['aria-labelledby'], first_dialog['aria-describedby']]

    find('body').send_keys(:escape)
    expect(page).to have_no_css('[role="dialog"]')

    find("[data-testid='take-dose-schedule_#{second_schedule.id}']").click
    second_dialog = find('dialog[open][role="dialog"]', text: 'Record dose')
    second_references = [second_dialog['aria-labelledby'], second_dialog['aria-describedby']]

    expect(second_references & first_references).to be_empty
  end
end
