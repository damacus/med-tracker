require "application_system_test_case"

class MedicinesTest < ApplicationSystemTestCase
  test "showing medicine with recommended dosages" do
    # Arrange
    medicine = medicines(:calpol_sixplus)

    # Act
    visit medicine_path(medicine)

    # Assert
    assert_selector "h1", text: "CALPOL® SixPlusTM Oral Suspension"
    assert_selector "h2", text: "Recommended Dosages by Age"

    # Check each age group's dosage is displayed
    within(".bg-white.rounded-lg.border") do
      # 6-8 years
      assert_selector ".p-4:nth-child(1)", text: "Children 6-8 years"
      assert_selector ".p-4:nth-child(1)", text: "5.0ml up to 4 times in 24 hours"

      # 8-10 years
      assert_selector ".p-4:nth-child(2)", text: "Children 8-10 years"
      assert_selector ".p-4:nth-child(2)", text: "7.5ml up to 4 times in 24 hours"

      # 10-12 years
      assert_selector ".p-4:nth-child(3)", text: "Children 10-12 years"
      assert_selector ".p-4:nth-child(3)", text: "10.0ml up to 4 times in 24 hours"

      # Over 12 years
      assert_selector ".p-4:nth-child(4)", text: "Adults and children over 12 years"
      assert_selector ".p-4:nth-child(4)", text: "15.0ml up to 4 times in 24 hours"
    end
  end
end
