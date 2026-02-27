# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People' do
  fixtures :accounts, :people, :users, :carer_relationships

  describe 'GET /people/new' do
    context 'when signed in as a parent' do
      before { sign_in(users(:jane)) }

      it 'renders dependent person types only' do
        get new_person_path

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('value="minor"')
        expect(response.body).to include('value="dependent_adult"')
        expect(response.body).not_to include('value="adult"')
      end
    end

    context 'when signed in as an admin' do
      before { sign_in(users(:admin)) }

      it 'rejects access' do
        get new_person_path

        expect(response).to redirect_to(root_path)
      end
    end
  end

  describe 'POST /people' do
    context 'when signed in as a parent' do
      let(:parent_user) { users(:jane) }

      before { sign_in(parent_user) }

      it 'creates a minor and auto-links carer relationship' do
        expect do
          post people_path, params: {
            person: {
              name: 'New Child',
              date_of_birth: 5.years.ago.to_date,
              person_type: 'minor'
            }
          }
        end.to change(Person, :count).by(1).and change(CarerRelationship, :count).by(1)

        created_person = Person.last
        expect(created_person.name).to eq('New Child')
        expect(created_person.has_capacity).to be false
        expect(created_person.user).to be_nil

        relationship = created_person.carer_relationships.first
        expect(relationship.carer).to eq(parent_user.person)
        expect(relationship.relationship_type).to eq('parent')
        expect(relationship.active).to be true
      end

      it 'creates a dependent adult and auto-links carer relationship' do
        expect do
          post people_path, params: {
            person: {
              name: 'Dependent Adult',
              date_of_birth: 70.years.ago.to_date,
              person_type: 'dependent_adult'
            }
          }
        end.to change(Person, :count).by(1)

        created_person = Person.last
        expect(created_person.has_capacity).to be false
        expect(created_person.carer_relationships.first.carer).to eq(parent_user.person)
      end

      it 'rejects creating an adult person' do
        expect do
          post people_path, params: {
            person: {
              name: 'Another Adult',
              date_of_birth: 30.years.ago.to_date,
              person_type: 'adult'
            }
          }
        end.not_to change(Person, :count)

        expect(response).to redirect_to(root_path)
      end
    end

    context 'when signed in as an admin' do
      before { sign_in(users(:admin)) }

      it 'rejects creating people' do
        expect do
          post people_path, params: {
            person: {
              name: 'Someone',
              date_of_birth: 10.years.ago.to_date,
              person_type: 'minor'
            }
          }
        end.not_to change(Person, :count)

        expect(response).to redirect_to(root_path)
      end
    end
  end

  describe 'POST /people with turbo_stream format' do
    before { sign_in(users(:jane)) }

    it 'returns turbo_stream and updates modal, people list, and flash on success' do
      post people_path,
           params: {
             person: {
               name: 'Turbo Child',
               date_of_birth: 6.years.ago.to_date,
               person_type: 'minor'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('target="people"')
      expect(response.body).to include('target="flash"')
    end

    it 'returns unprocessable content and re-renders modal on failure' do
      post people_path,
           params: {
             person: {
               name: '',
               date_of_birth: '',
               person_type: 'minor'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('person_form')
    end
  end

  describe 'PATCH /people/:id with turbo_stream format' do
    before { sign_in(users(:admin)) }

    it 'returns turbo_stream and updates card, show container, and flash on success' do
      person = people(:john)

      patch person_path(person),
            params: { person: { name: 'Turbo Updated Name' } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"person_#{person.id}\"")
      expect(response.body).to include("target=\"person_show_#{person.id}\"")
      expect(response.body).to include('target="flash"')
      expect(person.reload.name).to eq('Turbo Updated Name')
    end

    it 'returns unprocessable content and re-renders modal on failure' do
      person = people(:john)

      patch person_path(person),
            params: { person: { name: '', date_of_birth: '' } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('person_form')
    end
  end
end
