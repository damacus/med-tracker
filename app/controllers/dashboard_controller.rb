class DashboardController < ApplicationController
  def index
    @people = Person.includes(:prescriptions, :medicines).all
    @active_prescriptions = Prescription.active.includes(:person, :medicine)
    @upcoming_prescriptions = @active_prescriptions.group_by { |p| p.person }
  end
end
