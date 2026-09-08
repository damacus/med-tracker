require 'rails_helper'

RSpec.describe 'Medication pause context' do
  fixtures :accounts, :people, :users, :locations, :medications, :carer_relationships

  let(:person) { people(:child_user_person) }

  %w[schedule routine as_needed].each do |kind|
    context "with a #{kind} source" do
      let!(:source) do
        attributes = { person: person, medication: medications(:vitamin_d), dose_amount: 10, dose_unit: 'mg' }
        if kind == 'schedule'
          Schedule.create!(
            **attributes, frequency: 'Daily', start_date: Date.current, end_date: 1.year.from_now.to_date
          )
        else
          PersonMedication.create!(**attributes, administration_kind: kind)
        end
      end
      let(:pause_path) do
        if kind == 'schedule'
          pause_person_schedule_path(person, source)
        else
          pause_person_person_medication_path(person, source)
        end
      end
      let(:form_path) { "#{pause_path}_form" }
      let(:resume_path) { pause_path.sub(/pause\z/, 'resume') }

      before { sign_in(users(:parent)) }

      MedicationPausePeriod::PUBLIC_REASONS.each do |reason|
        it "accepts #{reason} without a note" do
          patch pause_path, params: { pause_period: { reason: reason } }

          expect(response).to have_http_status(:see_other)
          expect(source.medication_pause_periods.sole).to have_attributes(reason: reason, note: nil)
        end
      end

      it 'replaces the modal with an associated validation error for a Turbo submission' do
        patch pause_path, params: { pause_period: { note: 'Keep this note' } },
                          headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include('target="modal"', 'aria-describedby="pause-error"', 'Keep this note')
        expect(source.reload).not_to be_paused
      end

      it 'opens a labelled modal without pausing the source' do
        get form_path, headers: { 'Turbo-Frame' => 'modal' }

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('Pause medication', 'Reason', 'Note (optional)', 'Cancel')
        expect(response.body).to include('turbo-frame', 'ruby-ui--dialog')
        expect(source.reload).not_to be_paused
      end

      [nil, '', 'reason_not_recorded', 'invalid'].each do |reason|
        it "rejects unsupported reason #{reason.inspect} and preserves the note" do
          patch pause_path, params: { pause_period: { reason: reason, note: 'Awaiting delivery' } }

          expect(response).to have_http_status(:unprocessable_content)
          expect(response.body).to include('Choose a reason', 'Awaiting delivery')
          expect(source.reload).not_to be_paused
          expect(source.medication_pause_periods).to be_empty
        end
      end

      it 'records context at server acceptance and retains it after direct resume' do
        freeze_time do
          patch pause_path, params: {
            pause_period: { reason: 'out_of_supply', note: 'Awaiting delivery', started_at: 3.days.ago.iso8601 }
          }

          expect(response).to have_http_status(:see_other)
          period = source.medication_pause_periods.sole
          expect(period).to have_attributes(
            reason: 'out_of_supply', note: 'Awaiting delivery', started_at: Time.current
          )
          expect(period.recorded_by_membership.person).to eq(users(:parent).person)

          get person_path(person)
          expect(response.body).to include('Out of supply', 'Awaiting delivery', 'Pause history')

          patch resume_path
          expect(source.reload).not_to be_paused
          expect(period.reload.ended_at).to eq(Time.current)
          get person_path(person)
          expect(response.body).to include('Out of supply', 'Awaiting delivery', 'Resumed')
        end
      end

      it 'does not disclose a form or change state without manage access' do
        sign_in(users(:carer))
        get form_path
        expect(response).to redirect_to(root_path)
        patch pause_path, params: { pause_period: { reason: 'other' } }
        expect(response).to redirect_to(root_path)
        expect(source.reload).not_to be_paused
      end

      it 'retains completed pause context in the card after recording a dose' do
        patch pause_path, params: { pause_period: { reason: 'out_of_supply', note: 'Delivery arrived' } }
        patch resume_path

        expect do
          post pause_path.sub(/pause\z/, 'take_medication'),
               headers: { 'Accept' => 'text/vnd.turbo-stream.html' }
        end.to change(MedicationTake, :count).by(1)

        expect(response).to have_http_status(:ok)
        history = Nokogiri::HTML.fragment(response.body).css('details').text
        expect(history).to include(
          'Pause history', 'Out of supply', 'Delivery arrived', 'Resumed', users(:parent).person.name
        )
      end
    end
  end
end
