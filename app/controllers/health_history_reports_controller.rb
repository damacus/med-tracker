# frozen_string_literal: true

class HealthHistoryReportsController < ApplicationController
  def show
    authorize :report, :index?

    return redirect_to_reports_with_person_required unless selected_person

    authorize selected_person, :download_health_history?

    send_data pdf_body,
              filename: filename,
              type: 'application/pdf',
              disposition: 'attachment'
    record_download
  rescue Reports::GpDateRange::RangeTooLarge
    redirect_to reports_path, alert: t('reports.gp_date_range_too_large')
  rescue Reports::GpDateRange::EndBeforeStart
    redirect_to reports_path(start_date: params[:start_date], end_date: params[:end_date], person_id: params[:person_id])
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
    Reports::GpHealthHistoryQuery.new(person: selected_person, start_date: start_date, end_date: end_date).call
  end

  def people
    @people ||= policy_scope(Person).order(:name, :id)
  end

  def selected_person
    return @selected_person if defined?(@selected_person)

    person_id = params.permit(:person_id).fetch(:person_id, nil)
    return @selected_person = nil if person_id.blank? || !person_id.match?(/\A\d+\z/)

    @selected_person = manageable_people.find(person_id)
  end

  def manageable_people
    @manageable_people ||= PersonPolicy::Scope.new(pundit_user, people).resolve_for(:manage)
  end

  def start_date
    date_range.start_date
  end

  def end_date
    date_range.end_date
  end

  def date_range
    @date_range ||= Reports::GpDateRange.parse(start_date: params[:start_date], end_date: params[:end_date])
  end

  def filename
    "medtracker-health-history-#{start_date.iso8601}-to-#{end_date.iso8601}.pdf"
  end

  def redirect_to_reports_with_person_required
    redirect_to reports_path, alert: t('reports.health_history.person_required')
  end

  def record_download
    Audit::Event.record!(
      household: selected_person.household,
      request: request,
      event_type: 'health_history_report.downloaded',
      metadata: {
        person_id: selected_person.id,
        start_date: start_date.iso8601,
        end_date: end_date.iso8601,
        include_medication_takes: false,
        outcome: 'success'
      }
    )
  end
end
