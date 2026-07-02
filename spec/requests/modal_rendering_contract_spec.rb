# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Modal rendering contract' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages,
           :carer_relationships

  describe 'standalone HTML requests' do
    before { sign_in(users(:admin)) }

    it 'renders the people form with the application layout' do
      get new_person_path

      expect_application_layout
      expect(response.body).to include('New Person')
    end

    it 'renders the people edit form with the application layout' do
      get edit_person_path(people(:john))

      expect_application_layout
      expect(response.body).to include('Edit Person')
    end

    it 'renders the schedule form with the application layout' do
      get new_person_schedule_path(people(:john))

      expect_application_layout
      expect(response.body).to include('Add schedule for John Doe')
    end

    it 'renders the person medication form with the application layout' do
      get new_person_person_medication_path(people(:admin))

      expect_application_layout
      expect(response.body).to include('Add Medication for')
    end

    it 'renders the admin carer relationship form with the application layout' do
      get new_admin_carer_relationship_path

      expect_application_layout
      expect(response.body).to include('New Carer Relationship')
    end
  end

  describe 'standalone medication assignment HTML requests' do
    before { sign_in(users(:parent)) }

    it 'renders the medication assignment form with the application layout' do
      get new_person_medication_assignment_path(people(:child_user_person))

      expect_application_layout
      expect(response.body).to include('Add Medication for')
    end
  end

  describe 'modal frame HTML requests' do
    before { sign_in(users(:admin)) }

    it 'renders the people form as a layoutless modal fragment' do
      get new_person_path, headers: modal_frame_headers

      expect_modal_fragment
      expect(response.body).to include('New Person')
    end

    it 'renders the people edit form as a layoutless modal fragment' do
      get edit_person_path(people(:john)), headers: modal_frame_headers

      expect_modal_fragment
      expect(response.body).to include('Edit Person')
    end

    it 'renders the schedule form as a layoutless modal fragment' do
      get new_person_schedule_path(people(:john)), headers: modal_frame_headers

      expect_modal_fragment
      expect(response.body).to include('New Schedule for John Doe')
    end

    it 'renders the person medication form as a layoutless modal fragment' do
      get new_person_person_medication_path(people(:admin)), headers: modal_frame_headers

      expect_modal_fragment
      expect(response.body).to include('Add Medication for')
    end

    it 'renders the admin carer relationship form as a layoutless modal fragment' do
      get new_admin_carer_relationship_path, headers: modal_frame_headers

      expect_modal_fragment
      expect(response.body).to include('New Carer Relationship')
    end
  end

  describe 'modal medication assignment frame HTML requests' do
    before { sign_in(users(:parent)) }

    it 'renders the medication assignment form as a layoutless modal fragment' do
      get new_person_medication_assignment_path(people(:child_user_person)), headers: modal_frame_headers

      expect_modal_fragment
      expect(response.body).to include('Add Medication for')
    end
  end

  describe 'Turbo Stream modal requests' do
    before { sign_in(users(:admin)) }

    it 'replaces the modal frame for the people form' do
      get new_person_path, headers: turbo_stream_headers

      expect_modal_stream
      expect(response.body).to include('New Person')
    end

    it 'replaces the modal frame for the people edit form' do
      get edit_person_path(people(:john)), headers: turbo_stream_headers

      expect_modal_stream
      expect(response.body).to include('Edit Person')
    end

    it 'replaces the modal frame for the schedule form' do
      get new_person_schedule_path(people(:john)), headers: turbo_stream_headers

      expect_modal_stream
      expect(response.body).to include('New Schedule for John Doe')
    end

    it 'replaces the modal frame for the person medication form' do
      get new_person_person_medication_path(people(:admin)), headers: turbo_stream_headers

      expect_modal_stream
      expect(response.body).to include('Add Medication for')
    end

    it 'replaces the modal frame for the admin carer relationship form' do
      get new_admin_carer_relationship_path, headers: turbo_stream_headers

      expect_modal_stream
      expect(response.body).to include('New Carer Relationship')
    end
  end

  describe 'Turbo Stream medication assignment modal requests' do
    before { sign_in(users(:parent)) }

    it 'replaces the modal frame for the medication assignment form' do
      get new_person_medication_assignment_path(people(:child_user_person)), headers: turbo_stream_headers

      expect_modal_stream
      expect(response.body).to include('Add Medication for')
    end
  end

  def expect_application_layout
    expect_ok_html_response
    expect(response.body).to include('<html')
    expect(response.body).to include('stylesheet')
    expect(response.body).to include('data-controller="global-search responsive-shell"')
  end

  def expect_modal_fragment
    expect_ok_html_response
    expect(response.body).to include('<turbo-frame id="modal"')
    expect(response.body).not_to include('<html')
    expect(response.body).not_to include('stylesheet')
  end

  def expect_modal_stream
    expect_ok_turbo_stream_response
    expect(response.body).to include('action="replace"')
    expect(response.body).to include('target="modal"')
  end

  def modal_frame_headers
    { 'Turbo-Frame' => 'modal' }
  end

  def turbo_stream_headers
    { 'Accept' => 'text/vnd.turbo-stream.html' }
  end

  def expect_ok_html_response
    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('text/html')
  end

  def expect_ok_turbo_stream_response
    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('text/vnd.turbo-stream.html')
  end
end
