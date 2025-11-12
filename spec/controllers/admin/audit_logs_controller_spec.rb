# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::AuditLogsController do
  fixtures :users, :people, :sessions

  let(:admin) { users(:admin) }
  let(:admin_session) { sessions(:admin_session) }

  before do
    Current.session = admin_session
    PaperTrail.request.whodunnit = admin.id
    allow(controller).to receive(:current_user).and_return(admin)
  end

  after do
    Current.reset
    PaperTrail.request.whodunnit = nil
  end

  describe 'GET #index' do
    it 'returns http success for administrators' do
      get :index
      expect(response).to have_http_status(:success)
    end

    it 'assigns @versions' do
      # Create some test versions
      person = people(:john)
      person.update!(name: 'Updated Name')

      get :index
      expect(assigns(:versions)).to be_present
    end

    it 'filters by item_type when provided' do
      person = people(:john)
      person.update!(name: 'Updated Name')

      get :index, params: { item_type: 'Person' }
      expect(assigns(:versions).map(&:item_type).uniq).to eq(['Person'])
    end

    it 'filters by event when provided' do
      person = people(:john)
      person.update!(name: 'Updated Name')

      get :index, params: { event: 'update' }
      expect(assigns(:versions).map(&:event).uniq).to eq(['update'])
    end

    context 'when user is not an administrator' do
      let(:carer_person) { Person.create!(name: 'Carer User', date_of_birth: 30.years.ago) }
      let(:carer) do
        User.create!(
          email_address: 'carer@example.com',
          password: 'password',
          role: :carer,
          person: carer_person
        )
      end
      let(:carer_session) { Session.create!(user: carer, user_agent: 'Test', ip_address: '127.0.0.1') }

      before do
        Current.session = carer_session
        allow(controller).to receive(:current_user).and_return(carer)
      end

      it 'raises Pundit::NotAuthorizedError' do
        expect do
          get :index
        end.to raise_error(Pundit::NotAuthorizedError)
      end
    end
  end

  describe 'GET #show' do
    it 'returns http success for administrators' do
      person = people(:john)
      person.update!(name: 'Updated Name')
      version = person.versions.last

      get :show, params: { id: version.id }
      expect(response).to have_http_status(:success)
    end

    it 'assigns @version' do
      person = people(:john)
      person.update!(name: 'Updated Name')
      version = person.versions.last

      get :show, params: { id: version.id }
      expect(assigns(:version)).to eq(version)
    end

    context 'when user is not an administrator' do
      let(:carer_person) { Person.create!(name: 'Carer User', date_of_birth: 30.years.ago) }
      let(:carer) do
        User.create!(
          email_address: 'carer2@example.com',
          password: 'password',
          role: :carer,
          person: carer_person
        )
      end
      let(:carer_session) { Session.create!(user: carer, user_agent: 'Test', ip_address: '127.0.0.1') }

      before do
        Current.session = carer_session
        allow(controller).to receive(:current_user).and_return(carer)
      end

      it 'raises Pundit::NotAuthorizedError' do
        person = people(:john)
        person.update!(name: 'Updated Name')
        version = person.versions.last

        expect do
          get :show, params: { id: version.id }
        end.to raise_error(Pundit::NotAuthorizedError)
      end
    end
  end
end
