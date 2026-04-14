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
    report_data = Reports::IndexQuery.new(people: @people, start_date: @start_date, end_date: @end_date).call
    @daily_data = report_data.daily_data
    @inventory_alerts = report_data.inventory_alerts

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
end
