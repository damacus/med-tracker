# frozen_string_literal: true

class ReportsController < ApplicationController
  def index
    authorize :report, :index?

    # Resolve date range
    @end_date = params[:end_date].present? ? Date.parse(params[:end_date]) : Time.zone.today
    @start_date = params[:start_date].present? ? Date.parse(params[:start_date]) : @end_date - 6.days

    # Aggregating data for the current user and their patients
    # We use PersonPolicy::Scope to fetch people the user is authorized to see
    @people = policy_scope(Person)

    # Calculate real compliance for the date range
    @daily_data = calculate_daily_compliance(@people, @start_date, @end_date)

    @inventory_alerts = calculate_inventory_alerts(@people)

    render Views::Reports::Index.new(
      daily_data: @daily_data,
      inventory_alerts: @inventory_alerts,
      start_date: @start_date,
      end_date: @end_date
    )
  rescue ArgumentError
    # rubocop:disable Rails/I18nLocaleTexts
    redirect_to reports_path, alert: 'Invalid date format provided.'
    # rubocop:enable Rails/I18nLocaleTexts
  end

  private

  # rubocop:disable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
  def calculate_daily_compliance(people, start_date, end_date)
    person_ids = people.map(&:id)

    # Pre-fetch takes and prescriptions to avoid N+1
    takes = MedicationTake.where(prescription_id: Prescription.where(person_id: person_ids).select(:id))
                          .where(taken_at: start_date.beginning_of_day..end_date.end_of_day)
                          .group_by { |t| t.taken_at.to_date }

    prescriptions = Prescription.where(person_id: person_ids)
                                .where('start_date <= ? AND (end_date IS NULL OR end_date >= ?)', end_date, start_date)
                                .to_a

    (start_date..end_date).map do |date|
      active_prescriptions = prescriptions.select do |p|
        p.start_date <= date && (p.end_date.nil? || p.end_date >= date)
      end

      expected_doses = active_prescriptions.sum { |p| p.max_daily_doses || 1 }
      actual_doses = takes[date]&.size || 0

      percentage = if expected_doses.zero?
                     100 # No meds expected = 100% compliance
                   else
                     [(actual_doses.to_f / expected_doses * 100).round, 100].min
                   end

      {
        date: date,
        day_name: date.strftime('%a'),
        percentage: percentage,
        expected: expected_doses,
        actual: actual_doses
      }
    end
  end
  # rubocop:enable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity

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
