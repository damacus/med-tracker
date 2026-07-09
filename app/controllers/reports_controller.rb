# frozen_string_literal: true

class ReportsController < ApplicationController
  def index
    authorize :report, :index?

    # Resolve date range
    @date_range = Reports::DateRange.parse(start_date: params[:start_date], end_date: params[:end_date])
    @start_date = @date_range.start_date
    @end_date = @date_range.end_date

    # Aggregating data for the current user and their patients
    # We use PersonPolicy::Scope to fetch people the user is authorized to see
    @people = policy_scope(Person).order(:name, :id)
    @selected_person_id = params[:person_id].presence
    @filtered_people = filtered_people
    report_data = Reports::IndexQuery.new(people: @filtered_people, start_date: @start_date, end_date: @end_date).call
    today_taken_medications = Reports::TodayTakenMedicationsQuery.new(people: @filtered_people).call
    smart_insights = SmartInsights::IndexQuery.new(people: @filtered_people, start_date: @start_date, end_date: @end_date).call
    @daily_data = report_data.daily_data
    @inventory_alerts = report_data.inventory_alerts

    render Views::Reports::Index.new(
      daily_data: @daily_data,
      smart_insights: smart_insights,
      start_date: @start_date,
      end_date: @end_date,
      today_taken_medications: today_taken_medications,
      people: @people,
      selected_person_id: @selected_person_id
    )
  rescue Reports::DateRange::RangeTooLarge
    redirect_to reports_path, alert: t('reports.date_range_too_large')
  rescue ArgumentError
    redirect_to reports_path, alert: t('reports.invalid_date')
  end

  private

  def filtered_people
    return @people if @selected_person_id.blank?
    return @people.none unless @selected_person_id.match?(/\A\d+\z/)

    @people.where(id: @selected_person_id)
  end
end
