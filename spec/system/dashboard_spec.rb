# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Dashboard" do
  fixtures(
    :accounts,
    :users,
    :locations,
    :medications,
    :dosages,
    :schedules,
    :people,
    :carer_relationships,
    :person_medications,
    :medication_takes
  )

  it "loads the dashboard and allows taking a dose from the timeline" do
    driven_by(:playwright)

    travel_to(Time.current.beginning_of_day + 9.hours) do
      sign_in(users(:jane))
      visit(dashboard_path)

      expect(page).to(have_text("Good morning"))
      expect(page).to(have_text("Today's Schedule"))
      expect(page).to(have_text("Ibuprofen"))
      expect(page).to(have_text("Jane Doe"))
      expect(page).to(have_text("Child Patient"))

      button = first("[data-testid^=\"take-dose-\"]")

      expect do
        button.click
        within(first("form[action*='take_medication']", visible: :all)) do
          click_button(I18n.t("person_medications.card.take"))
        end

        expect(page).to(have_text("Medication taken successfully.", wait: 10))
      end
        .to(change(MedicationTake, :count).by(1))
    end
  end
end
