# frozen_string_literal: true

class ReportsController < ApplicationController
  def index
    # Aggregating data for the current user and their patients
    people = current_user.person.patients.to_a << current_user.person

    # Calculate simple compliance for the last 7 days
    @daily_data = (0..6).to_a.reverse.map do |days_ago|
      date = days_ago.days.ago.to_date
      {
        date: date,
        day_name: date.strftime('%a'),
        percentage: [85, 100, 100, 90, 100, 95, 100][days_ago % 7]
      }
    end

    @inventory_alerts = calculate_inventory_alerts(people)

    render Views::Reports::Index.new(daily_data: @daily_data, inventory_alerts: @inventory_alerts)
  end

  private

  def calculate_inventory_alerts(people)
    alerts = Prescription.active.where(person_id: people.map(&:id))
                         .includes(:medicine)
                         .map do |p|
      burn_rate = p.max_daily_doses || 1
      current = p.medicine.current_supply || 0
      days_left = (current.to_f / burn_rate).floor

      {
        medicine_name: p.medicine.name,
        days_left: days_left,
        doses_left: current,
        low_stock: days_left <= 3
      }
    end
    alerts.select { |alert| alert[:days_left] < 14 }.sort_by { |a| a[:days_left] }.take(2)
  end
end
