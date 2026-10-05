require 'rails_helper'

RSpec.describe 'API v1 profile avatars' do
  fixtures :all

  let(:login) { api_login(users(:jane)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:person) { users(:jane).person }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/profile/avatar" }

  before { login }

  it 'advertises protected online avatar operations and upload limits' do
    get '/api/v1/capabilities', as: :json
    expect(response.parsed_body.dig('data', 'profile', 'avatar')).to include(
      'actions' => %w[show update destroy], 'max_bytes' => 5.megabytes,
      'content_types' => %w[image/png image/jpeg image/webp]
    )
  end

  def upload(contents = 'avatar', content_type = 'image/png', filename = 'avatar.png')
    file = Tempfile.new(['avatar', File.extname(filename)])
    file.binmode
    file.write(contents)
    file.rewind
    Rack::Test::UploadedFile.new(file.path, content_type, original_filename: filename)
  end

  def attach_original
    person.avatar.attach(io: StringIO.new('original'), filename: 'original.png', content_type: 'image/png')
  end

  it 'uploads and reads the current person image through an authenticated route' do
    put path, params: { avatar: upload }, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'avatar_attached')).to be(true)
    get path, headers: headers
    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('image/png')
    expect(response.headers['Cache-Control']).to include('no-store')
    expect(response.body).to eq('avatar')
    get path
    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects unsupported and oversized files while retaining the original attachment' do
    attach_original
    [upload('text', 'text/plain', 'avatar.txt'), upload('x' * 6.megabytes)].each do |file|
      put path, params: { avatar: file }, headers: headers
      expect(response).to have_http_status(:unprocessable_content)
      expect(person.reload.avatar.download).to eq('original')
    end
  end

  it 'rejects a signed blob identifier instead of attaching another person file' do
    attach_original
    put path, params: { avatar: person.avatar.blob.signed_id }, headers: headers
    expect(response).to have_http_status(:unprocessable_content)
    expect(person.reload.avatar.download).to eq('original')
  end

  it 'removes the image and makes subsequent reads unavailable' do
    attach_original
    delete path, headers: headers
    expect(response).to have_http_status(:no_content)
    expect(person.reload.avatar).not_to be_attached
    get path, headers: headers
    expect(response).to have_http_status(:not_found)
  end

  it 'enforces revoked person access for reads and writes' do
    attach_original
    membership = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: membership, person: person).destroy_all
    get path, headers: headers
    expect(response).to have_http_status(:forbidden)
    put path, params: { avatar: upload }, headers: headers
    expect(response).to have_http_status(:forbidden)
    delete path, headers: headers
    expect(response).to have_http_status(:forbidden)
    expect(person.reload.avatar.download).to eq('original')
  end

  it 'keeps the original attachment when object storage rejects an upload' do
    attach_original
    allow(Observability::EmergencyDiagnostic).to receive(:write).and_call_original
    allow(ActiveStorage::Blob.service).to receive(:upload).and_raise(IOError, 'Private storage credentials')
    put path, params: { avatar: upload }, headers: headers
    expect(response).to have_http_status(:service_unavailable)
    expect(response.body).not_to include('Private storage credentials')
    expect(Observability::EmergencyDiagnostic).not_to have_received(:write)
    expect(person.reload.avatar.download).to eq('original')
  end

  it 'returns a stable error when stored image bytes are unavailable' do
    attach_original
    person.avatar.blob.service.delete(person.avatar.blob.key)
    get path, headers: headers
    expect(response).to have_http_status(:service_unavailable)
    expect(response.parsed_body.dig('error', 'code')).to eq('avatar_unavailable')
  end
end
