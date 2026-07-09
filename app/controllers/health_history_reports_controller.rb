# frozen_string_literal: true

class HealthHistoryReportsController < ApplicationController
  def show
    authorize :report, :index?

    send_data pdf_body,
              filename: filename,
              type: 'application/pdf',
              disposition: 'attachment'
  rescue Reports::DateRange::RangeTooLarge
    redirect_to reports_path, alert: t('reports.date_range_too_large')
  rescue ArgumentError
    redirect_to reports_path, alert: t('reports.invalid_date')
  end

  private

  def pdf_body
    response.headers['Cache-Control'] = 'no-store'
    Reports::HealthHistoryPdf.new(
      result: report_result,
      start_date: start_date,
      end_date: end_date,
      generated_at: Time.current
    ).render
  end

  def report_result
    Reports::HealthHistoryQuery.new(people: filtered_people, start_date: start_date, end_date: end_date).call
  end

  def people
    @people ||= policy_scope(Person).order(:name, :id)
  end

  def filtered_people
    person_id = params.permit(:person_id).fetch(:person_id, nil)
    return people if person_id.blank?
    return people.none unless person_id.match?(/\A\d+\z/)

    people.where(id: person_id)
  end

  def start_date
    date_range.start_date
  end

  def end_date
    date_range.end_date
  end

  def date_range
    @date_range ||= Reports::DateRange.parse(start_date: params[:start_date], end_date: params[:end_date])
  end

  def filename
    "medtracker-health-history-#{start_date.iso8601}-to-#{end_date.iso8601}.pdf"
  end
end
