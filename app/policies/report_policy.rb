# frozen_string_literal: true

class ReportPolicy < ApplicationPolicy
  def index?
    admin? || medical_staff? || carer_or_parent? || user.person.adult?
  end
end
