require 'rails_helper'

RSpec.describe Api::V1::BaseController, type: :controller do
  controller(Api::V1::BaseController) do
    def index
      render json: { success: true }
    end

    def show
      raise ActiveRecord::RecordNotFound
    end

    def create
      raise Api::V1::BaseController::InvalidFilterValue, 'invalid filter'
    end

    def update
      raise Pundit::NotAuthorizedError
    end

    def new
      render_unauthorized('custom unauthorized')
    end

    def edit
      render_forbidden
    end
  end

  let(:account) { instance_double(Account, present?: true, verified?: true) }
  let(:user) { instance_double(User, present?: true, active?: true) }
  let(:person) { instance_double(Person, user: user) }
  let(:api_session) do
    instance_double(ApiSession,
                    blank?: false,
                    is_a?: true,
                    revoked_at: nil,
                    access_expires_at: 1.day.from_now,
                    account: account,
                    touch_last_used!: true)
  end

  before do
    allow(account).to receive(:person).and_return(person)
    allow(ApiSession).to receive(:lookup_by_access_token).and_return(api_session)
    allow(ApiAppToken).to receive(:lookup_by_token).and_return(nil)
    allow(ApiAuthState).to receive(:locked_out?).and_return(false)
    allow(api_session).to receive(:is_a?).with(ApiSession).and_return(true)

    request.headers['Authorization'] = 'Bearer valid_token'
  end

  describe 'authentication via around_action :with_api_request_context' do
    context 'with valid token' do
      it 'allows the request and touches the session' do
        get :index
        expect(response).to have_http_status(:ok)
        expect(response.parsed_body).to eq({ 'success' => true })
        expect(api_session).to have_received(:touch_last_used!)
      end
    end

    context 'with missing token' do
      before do
        request.headers['Authorization'] = nil
        allow(ApiSession).to receive(:lookup_by_access_token).and_return(nil)
      end

      it 'returns unauthorized' do
        get :index
        expect(response).to have_http_status(:unauthorized)
        expect(response.parsed_body).to eq({ 'error' => { 'code' => 'unauthorized', 'message' => 'Authentication required' } })
      end
    end

    context 'with expired token' do
      let(:api_session) do
        instance_double(ApiSession,
                        blank?: false,
                        revoked_at: nil,
                        access_expires_at: 1.day.ago)
      end

      before do
        allow(api_session).to receive(:is_a?).with(ApiSession).and_return(true)
      end

      it 'returns unauthorized' do
        get :index
        expect(response).to have_http_status(:unauthorized)
      end
    end

    context 'with revoked token' do
      let(:api_session) do
        instance_double(ApiSession,
                        blank?: false,
                        revoked_at: Time.current)
      end

      it 'returns unauthorized' do
        get :index
        expect(response).to have_http_status(:unauthorized)
      end
    end

    context 'when account is locked out' do
      before do
        allow(ApiAuthState).to receive(:locked_out?).with(account).and_return(true)
      end

      it 'returns unauthorized' do
        get :index
        expect(response).to have_http_status(:unauthorized)
      end
    end

    context 'when user is not active' do
      let(:user) { instance_double(User, present?: true, active?: false) }

      it 'returns unauthorized' do
        get :index
        expect(response).to have_http_status(:unauthorized)
      end
    end
  end

  describe 'exception handling' do
    it 'rescues ActiveRecord::RecordNotFound and returns 404' do
      get :show, params: { id: 1 }
      expect(response).to have_http_status(:not_found)
      expect(response.parsed_body).to eq({ 'error' => { 'code' => 'not_found', 'message' => 'Record not found' } })
    end

    it 'rescues InvalidFilterValue and returns 422' do
      post :create
      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body).to eq({ 'error' => { 'code' => 'unprocessable_content', 'message' => 'invalid filter' } })
    end

    it 'rescues Pundit::NotAuthorizedError and returns 403' do
      put :update, params: { id: 1 }
      expect(response).to have_http_status(:forbidden)
      expect(response.parsed_body).to eq({ 'error' => { 'code' => 'forbidden', 'message' => 'You are not authorized to perform this action.' } })
    end
  end

  describe 'explicit render methods' do
    it 'renders unauthorized correctly' do
      get :new
      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body).to eq({ 'error' => { 'code' => 'unauthorized', 'message' => 'custom unauthorized' } })
    end

    it 'renders forbidden correctly' do
      get :edit, params: { id: 1 }
      expect(response).to have_http_status(:forbidden)
      expect(response.parsed_body).to eq({ 'error' => { 'code' => 'forbidden', 'message' => 'You are not authorized to perform this action.' } })
    end
  end

  describe '#render_validation_errors' do
    let(:errors) { instance_double(ActiveModel::Errors, to_hash: { field: ['is invalid'] }) }
    let(:record) { instance_double(Person, errors: errors) }

    it 'returns unprocessable_content with errors hash' do
      controller.send(:render_validation_errors, record)
      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body).to eq({
        'error' => {
          'code' => 'validation_failed',
          'message' => 'Validation failed',
          'errors' => { 'field' => ['is invalid'] }
        }
      })
    end
  end
end
