# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Reports' do
  fixtures :all

  let(:user) { users(:admin) }

  describe 'GET /reports' do
    context 'when user is not authenticated' do
      it 'redirects to the login page' do
        get reports_path
        expect(response).to redirect_to(login_path)
      end
    end

    context 'when user is authenticated and authorized' do
      before do
        sign_in(user)
      end

      it 'returns HTTP success' do
        get reports_path
        expect(response).to have_http_status(:success)
      end

      it 'returns HTTP success when applying date filters' do
        get reports_path, params: { start_date: '2023-01-01', end_date: '2023-01-31' }
        expect(response).to have_http_status(:success)
      end

      it 'returns HTTP success and filters report content by accessible person' do
        get reports_path, params: { person_id: people(:john).id }

        expect(response).to have_http_status(:success)
        expect(response.body).to include('John Doe')
        expect(response.body).to include('Paracetamol')
        expect(response.body).not_to include('Ibuprofen')
      end

      it 'does not expose inaccessible person data through the person filter' do
        post '/logout'
        sign_in(users(:parent))

        get reports_path, params: { person_id: people(:john).id }

        expect(response).to have_http_status(:success)
        expect(response.body).not_to include('John Doe')
        expect(response.body).not_to include('Paracetamol')
      end

      it 'renders the Smart Insights anchor section' do
        get reports_path

        expect(response.body).to include('id="insights"')
        expect(response.body).to include('Smart Insights')
      end

      it 'renders the Smart Insights anchor with a section heading' do
        get reports_path

        page = response.parsed_body
        insights_section = page.at_css('section#insights')

        expect(insights_section).to be_present
        expect(page.css('#insights').size).to eq(1)
        expect(insights_section.at_css('h2')&.text).to include('Smart Insights')
      end

      it 'does not render the retired static achievement copy' do
        get reports_path

        expect(response.body).not_to include('Achievement Streak')
        expect(response.body).not_to include('4 Days Uninterrupted')
        expect(response.body).not_to include('maintaining optimal levels')
        expect(response.body).not_to include('Current Health Status')
      end

      it 'redirects with alert when providing invalid date formats' do
        get reports_path, params: { start_date: 'invalid-date' }
        expect(response).to redirect_to(reports_path)
        expect(flash[:alert]).to eq('Invalid date format provided.')
      end

      it 'renders a preserved filter error when the end date is before the start date' do
        person = people(:john)

        get reports_path, params: { start_date: '2026-02-01', end_date: '2026-01-01', person_id: person.id }

        expect(response).to have_http_status(:unprocessable_content)

        page = response.parsed_body
        expect(page.at_css('#start_date')['value']).to eq('2026-02-01')
        expect(page.at_css('#end_date')['value']).to eq('2026-01-01')
        expect(page.at_css('#person_id option[selected]')['value']).to eq(person.id.to_s)
        expect(page.at_css('[role="alert"]').text).to include('End date must be on or after start date.')
        expect(page.at_css("a[href*='/reports/health-history']")).to be_nil
      end

      it 'redirects with alert when the report date range exceeds 180 days' do
        get reports_path, params: { start_date: '2026-01-01', end_date: '2026-07-01' }

        expect(response).to redirect_to(reports_path)
        expect(flash[:alert]).to eq('Report date range cannot exceed 180 days.')
      end
    end

    context 'when user is authenticated but not authorized' do
      let(:user) { users(:minor_patient_user) } # minor

      before do
        sign_in(user)
      end

      it 'redirects to root path' do
        get reports_path
        expect(response).to redirect_to(root_path)
        expect(flash[:alert]).to eq('You are not authorized to perform this action.')
      end
    end
  end

  describe 'GET /reports/health-history' do
    before { sign_in(user) }

    it 'redirects safely when PDF rendering fails' do
      renderer = instance_double(Reports::HealthHistoryPdf)
      allow(Reports::HealthHistoryPdf).to receive(:new).and_return(renderer)
      allow(renderer).to receive(:render).and_raise(Reports::PdfRenderer::Error, 'rendering failed')
      allow(Rails.logger).to receive(:error)

      get health_history_report_path,
          params: { start_date: '2026-02-01', end_date: '2026-02-28', person_id: people(:john).id }

      expect(response).to redirect_to(
        reports_path(start_date: '2026-02-01', end_date: '2026-02-28', person_id: people(:john).id)
      )
      expect(response.media_type).not_to eq('application/pdf')
      expect(response.body).not_to start_with('%PDF')
      expect(flash[:alert]).to eq(I18n.t('reports.export.pdf_unavailable'))
      expect(Rails.logger).to have_received(:error).with(
        include('report_type=health_history', 'exception_class=Reports::PdfRenderer::Error')
      )
    end

    it 'downloads a no-store PDF with the active filters' do
      get health_history_report_path,
          params: { start_date: '2026-02-01', end_date: '2026-02-28', person_id: people(:john).id }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('application/pdf')
      expect(response.headers['Cache-Control']).to include('no-store')
      expect(response.headers['Content-Disposition'])
        .to include('medtracker-health-history-2026-02-01-to-2026-02-28.pdf')
      expect(response.body).to start_with('%PDF')
      expect(pdf_text(response.body)).to include('People:', 'Date range:', 'Medication administrations')
    end

    it 'does not export an inaccessible person filter' do
      post '/logout'
      sign_in(users(:parent))

      get health_history_report_path, params: { person_id: people(:john).id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to start_with('%PDF')
    end

    it 'exports an empty PDF for nonnumeric person filters' do
      get health_history_report_path, params: { person_id: 'not-a-person' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to start_with('%PDF')
    end

    it 'redirects with alert when export dates are invalid' do
      get health_history_report_path, params: { start_date: 'invalid-date' }

      expect(response).to redirect_to(reports_path)
      expect(flash[:alert]).to eq('Invalid date format provided.')
    end

    it 'returns reversed export filters to the report form with one dedicated alert' do
      person = people(:john)

      get health_history_report_path,
          params: { start_date: '2026-02-01', end_date: '2026-01-01', person_id: person.id }

      expect(response).to redirect_to(
        reports_path(start_date: '2026-02-01', end_date: '2026-01-01', person_id: person.id)
      )

      follow_redirect!

      expect(response).to have_http_status(:unprocessable_content)

      alerts = response.parsed_body.css('[role="alert"]')
      expect(alerts.size).to eq(1)
      expect(alerts.first.text).to include('End date must be on or after start date.')
    end

    it 'redirects with alert when the PDF date range exceeds 180 days' do
      get health_history_report_path, params: { start_date: '2026-01-01', end_date: '2026-07-01' }

      expect(response).to redirect_to(reports_path)
      expect(flash[:alert]).to eq('Report date range cannot exceed 180 days.')
    end
  end
end
