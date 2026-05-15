# frozen_string_literal: true

class ReportsController < ApplicationController
  def index
    authorize :report, :index?

    # Resolve date range
    @end_date = params[:end_date].present? ? Date.parse(params[:end_date]) : Time.zone.today
    @start_date = params[:start_date].present? ? Date.parse(params[:start_date]) : @end_date - 6.days

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
  rescue ArgumentError
    # rubocop:disable Rails/I18nLocaleTexts
    redirect_to reports_path, alert: 'Invalid date format provided.'
    # rubocop:enable Rails/I18nLocaleTexts
  end

  private

  def filtered_people
    return @people if @selected_person_id.blank?
    return @people.none unless @selected_person_id.match?(/\A\d+\z/)

    @people.where(id: @selected_person_id)
  end
end
