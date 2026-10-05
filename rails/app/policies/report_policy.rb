# frozen_string_literal: true

class ReportPolicy < ApplicationPolicy
  def index?
    household_report_access?
  end

  private

  def household_report_access?
    active_membership? && (household_manager? || membership.person&.adult?)
  end
end
