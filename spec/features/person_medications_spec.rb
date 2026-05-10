# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Person Medications", type: :system do
  fixtures(:accounts, :people, :locations, :location_memberships, :medications, :users, :person_medications)

  let(:person) { people(:john) }
  let(:user) { users(:john) }

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
    login_as(user)
  end

  describe "adding a medication assignment" do
    let(:new_medication) { medications(:vitamin_c) }

    it "allows adding a medication from predefined defaults", :js do
      visit(person_path(person))

      expect(page).to(have_text("Medications"))

      within("[data-testid=\"quick-actions\"]") do
        click_link("Add Medication")
      end

      click_button("Select a medication")
      find("label", text: new_medication.name).click

      expect(page).to(have_text("Choose the dose"))
      expect(page).to(have_css("#medication_assignment_dose_option option", text: "500 mg", visible: :all))
      select("500 mg", from: "Dose")
      click_button("Next")

      expect(page).to(have_text("Review"))

      click_button("Add Medication")

      expect(page).to(have_text("Schedule was successfully created."))
      expect(page).to(have_text(new_medication.name))
    end
  end

  describe "recording medication takes" do
    let(:person_medication) { person_medications(:john_vitamin_d) }

    it "allows recording a medication take", :js do
      person_medication.update!(max_daily_doses: 3, min_hours_between_doses: nil)
      person_medication.medication_takes.delete_all

      visit(person_path(person))

      within("#person_medication_#{person_medication.id}") do
        click_button("Take")
      end

      confirm_record_dose(person_medication)

      expect(page).to(have_text("Medication taken successfully"))
    end

    it "rejects the default dose time when max daily doses reached", :js do
      person_medication.update!(max_daily_doses: 2, min_hours_between_doses: nil)
      person_medication.medication_takes.delete_all
      2.times do
        MedicationTake.create!(
          person_medication: person_medication,
          taken_at: Time.current,
          dose_amount: 5
        )
      end

      visit(person_path(person))

      within("#person_medication_#{person_medication.id}") do
        expect(page).to(have_button("Take", disabled: false))
        click_button("Take")
      end

      expect do
        confirm_record_dose(person_medication)
        expect(page).to(have_text("Cannot take medication"))
      end
        .not_to(change(MedicationTake, :count))
    end

    it "rejects the default dose time when minimum hours not passed", :js do
      person_medication.update!(max_daily_doses: nil, min_hours_between_doses: 6)
      person_medication.medication_takes.delete_all
      MedicationTake.create!(
        person_medication: person_medication,
        taken_at: 2.hours.ago,
        dose_amount: 5
      )

      visit(person_path(person))

      within("#person_medication_#{person_medication.id}") do
        expect(page).to(have_button("Take", disabled: false))
        click_button("Take")
      end

      expect do
        confirm_record_dose(person_medication)
        expect(page).to(have_text("Cannot take medication"))
      end
        .not_to(change(MedicationTake, :count))
    end
  end

  describe "viewing today's doses on card" do
    let(:person_medication) { person_medications(:john_vitamin_d) }

    let!(:take) do
      MedicationTake.create!(
        person_medication: person_medication,
        taken_at: Time.current,
        dose_amount: 5
      )
    end

    it "displays today's doses on the medication card" do
      visit(person_path(person))

      within("#person_medication_#{person_medication.id}") do
        expect(page).to(have_text(/today's doses/i))
        expect(page).to(have_text(take.taken_at.strftime("%l:%M %p").strip))
        expect(page).to(have_text("5 IU"))
      end
    end
  end

  def confirm_record_dose(person_medication)
    path = take_medication_person_person_medication_path(person, person_medication)

    within("form[action='#{path}']") do
      click_button("Take")
    end
  end
end
