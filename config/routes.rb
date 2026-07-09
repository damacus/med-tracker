# frozen_string_literal: true

Rails.application.routes.draw do
  mount MedTrackerMcp::RackApp.new => '/mcp'

  namespace :api do
    scope 'fhir/R4', module: 'fhir/r4', as: :fhir_r4 do
      get '.well-known/smart-configuration', to: 'smart_configuration#show'
      get :metadata, to: 'metadata#show'
      get 'Patient', to: 'patients#index'
      get 'Patient/:id', to: 'patients#show'
      get 'Medication', to: 'medications#index'
      get 'Medication/:id', to: 'medications#show'
      get 'MedicationRequest', to: 'medication_requests#index'
      get 'MedicationRequest/:id', to: 'medication_requests#show'
      get 'MedicationStatement', to: 'medication_statements#index'
      get 'MedicationStatement/:id', to: 'medication_statements#show'
      get 'MedicationAdministration', to: 'medication_administrations#index'
      get 'MedicationAdministration/:id', to: 'medication_administrations#show'
    end

    namespace :v1 do
      get :capabilities, to: 'capabilities#show'

      namespace :auth do
        post :login, to: 'sessions#create'
        post :oidc_exchange, to: 'sessions#oidc_exchange'
        get :households, to: 'sessions#households'
        get :sessions, to: 'sessions#index'
        delete 'sessions/:id', to: 'sessions#revoke', as: :session
        post :refresh, to: 'sessions#refresh'
        delete :logout, to: 'sessions#destroy'
      end

      scope 'households/:household_id', as: :household do
        resource :me, only: [:show], controller: 'me'
        resources :people, only: %i[index show create update]
        resources :locations, only: %i[index show]
        resources :medications, only: %i[index show create update]
        resources :medications, only: [] do
          member do
            patch :adjust_inventory
            patch :mark_as_ordered
            patch :mark_as_received
          end
        end
        resources :dosage_options, only: %i[index show create update]
        resources :health_events, only: %i[index show create update]
        resources :schedules, only: %i[index show create update] do
          member do
            patch :pause
            patch :resume
          end
        end
        resources :person_medications, only: %i[index show create update] do
          member do
            patch :pause
            patch :resume
            patch :reorder
          end
        end
        resources :medication_takes, only: %i[index create]
        resource :notification_preference, only: %i[show update]
        resources :native_device_tokens, only: %i[create destroy]
        resource :push_subscription, only: %i[create destroy] do
          post :test, on: :collection
        end
        get :medication_lookup, to: 'medication_lookup#show'
        post :ai_medication_suggestions, to: 'ai_medication_suggestions#create'
        get :portable_export, to: 'portable_exports#show'
        get :mobile_snapshot, to: 'mobile_snapshots#show'
        post 'portable_imports/dry_run', to: 'portable_imports#dry_run'
        post :portable_imports, to: 'portable_imports#create'
        get 'data_exports/:mode', to: 'data_exports#show', as: :data_export

        namespace :sync do
          get :snapshot, to: 'feeds#snapshot'
          get :changes, to: 'feeds#changes'
          post :batches, to: 'batches#create'
        end

        namespace :admin do
          resource :settings, only: %i[show update], controller: 'settings'
          resources :memberships, only: %i[index update destroy]
          resources :invitations, only: %i[index create destroy]
          resources :person_access_grants, only: %i[index create destroy]
          resources :app_tokens, only: %i[index create destroy]
          resources :audit_logs, only: %i[index]
        end
      end
    end
  end

  scope ActiveStorage.routes_prefix do
    get '/blobs/redirect/:signed_id/*filename',
        to: 'active_storage/blobs/redirect#show', as: :rails_service_blob
    get '/blobs/proxy/:signed_id/*filename',
        to: 'active_storage/blobs/proxy#show', as: :rails_service_blob_proxy
    get '/blobs/:signed_id/*filename', to: 'active_storage/blobs/redirect#show'
    get '/representations/redirect/:signed_blob_id/:variation_key/*filename',
        to: 'active_storage/representations/redirect#show', as: :rails_blob_representation
    get '/representations/proxy/:signed_blob_id/:variation_key/*filename',
        to: 'active_storage/representations/proxy#show', as: :rails_blob_representation_proxy
    get '/representations/:signed_blob_id/:variation_key/*filename',
        to: 'active_storage/representations/redirect#show'
    get '/disk/:encoded_key/*filename', to: 'active_storage/disk#show', as: :rails_disk_service
  end

  direct :rails_representation do |representation, options|
    route_for(ActiveStorage.resolve_model_to_route, representation, options)
  end

  resolve('ActiveStorage::Variant') do |variant, options|
    route_for(ActiveStorage.resolve_model_to_route, variant, options)
  end

  resolve('ActiveStorage::VariantWithRecord') do |variant, options|
    route_for(ActiveStorage.resolve_model_to_route, variant, options)
  end

  resolve('ActiveStorage::Preview') do |preview, options|
    route_for(ActiveStorage.resolve_model_to_route, preview, options)
  end

  direct :rails_blob do |blob, options|
    route_for(ActiveStorage.resolve_model_to_route, blob, options)
  end

  resolve('ActiveStorage::Blob') do |blob, options|
    route_for(ActiveStorage.resolve_model_to_route, blob, options)
  end

  resolve('ActiveStorage::Attachment') do |attachment, options|
    route_for(ActiveStorage.resolve_model_to_route, attachment.blob, options)
  end

  direct :rails_storage_proxy do |model, options|
    expires_in = options.delete(:expires_in) { ActiveStorage.urls_expire_in }
    expires_at = options.delete(:expires_at)

    if model.respond_to?(:signed_id)
      route_for(
        :rails_service_blob_proxy,
        model.signed_id(expires_in:, expires_at:),
        model.filename,
        options
      )
    else
      route_for(
        :rails_blob_representation_proxy,
        model.blob.signed_id(expires_in:, expires_at:),
        model.variation.key,
        model.blob.filename,
        options
      )
    end
  end

  direct :rails_storage_redirect do |model, options|
    expires_in = options.delete(:expires_in) { ActiveStorage.urls_expire_in }
    expires_at = options.delete(:expires_at)

    if model.respond_to?(:signed_id)
      route_for(
        :rails_service_blob,
        model.signed_id(expires_in:, expires_at:),
        model.filename,
        options
      )
    else
      route_for(
        :rails_blob_representation,
        model.blob.signed_id(expires_in:, expires_at:),
        model.variation.key,
        model.blob.filename,
        options
      )
    end
  end

  # Defines the root path route ("/")
  root 'household_redirects#show'

  namespace :platform do
    resource :settings, only: %i[show update]
    resources :users, only: %i[index update]
    resources :support_access_sessions, only: %i[create destroy]
  end

  scope 'households/:household_slug' do
    get 'search', to: 'searches#show'

    get 'dashboard', to: 'dashboard#index'
    get 'add_medication', to: 'medication_workflow#index', as: :add_medication

    resource :profile, only: %i[show update] do
      patch :experiments, on: :member
      resources :api_tokens, only: %i[create destroy], controller: 'profiles/api_tokens'
    end
    get 'profile/data_exports/:mode', to: 'profiles/data_exports#show', as: :profile_data_export
    delete 'profile/avatar', to: 'profiles#avatar', as: :profile_avatar

    resources :reports, only: %i[index]
    get 'reports/health-history', to: 'health_history_reports#show', as: :health_history_report
    get 'medicine-reviews/report', to: 'medication_review_reports#show', as: :medication_review_report
    resources :medication_review_prompts, path: 'medicine-reviews', only: %i[index update]

    get 'offline', to: 'offline#show'
    get 'offline/snapshot', to: 'offline#snapshot'
    post 'offline/medication_takes', to: 'offline#medication_takes'

    resources :locations do
      resources :location_memberships, only: %i[create destroy]
    end

    post 'medications/scan_restock', to: 'medications#scan_restock', as: :scan_restock_medications
    get 'medications/scan_restock_match', to: 'medications#scan_restock_match', as: :scan_restock_match_medications
    resources :medications do
      member do
        get :administration
        get :nhs_guidance
        patch :refill
        patch :adjust_inventory
        patch :mark_as_ordered
        patch :mark_as_received
      end
    end
    get 'medication-finder', to: 'medications#finder', as: :medication_finder
    get 'medication-finder/search', to: 'medications#search', as: :medication_finder_search
    post 'ai-medication-suggestions', to: 'ai_medication_suggestions#create', as: :ai_medication_suggestions

    get 'schedules/workflow', to: 'schedules#workflow', as: :schedules_workflow
    post 'schedules/workflow', to: 'schedules#start_workflow', as: :start_schedules_workflow
    get 'schedules/frequency_preview', to: 'schedules#frequency_preview', as: :schedules_frequency_preview
    resources :schedules do
      resources :medication_takes, only: [:create]
    end

    get 'people/:person_id/avatar', to: 'people/avatars#show', as: :person_avatar
    resources :people do
      member do
        get :add_medication
      end

      resources :schedules, except: [:index] do
        member do
          patch :pause
          patch :resume
          post :take_medication
        end
      end

      resources :person_medications, except: [:index] do
        member do
          patch :pause
          patch :resume
          patch :reorder
          post :take_medication
        end
      end
      resources :carer_relationships, only: %i[new create], controller: 'people/carer_relationships'
      resources :medication_assignments, only: %i[new create]
      resources :health_events, except: [:show]
    end

    resource :push_subscription, only: %i[create destroy] do
      post :test, on: :collection
    end
    resources :native_device_tokens, only: %i[create destroy]
    resource :notification_preference, only: %i[update]

    namespace :admin do
      root to: 'dashboard#index'
      resource :household, only: %i[edit update]
      resource :nhs_dmd_import, only: %i[new create]
      resources :users, only: %i[index new create edit update destroy] do
        member do
          post :activate
          patch :membership_role, to: 'membership_roles#update'
          post :verify
        end
      end
      resources :invitations, only: %i[index create destroy] do
        member do
          post :resend
        end
      end
      resources :carer_relationships, only: %i[index new create destroy] do
        member do
          post :activate
        end
      end
      resources :people, only: %i[index]
      resources :audit_logs, only: %i[index show]
      resource :settings, only: %i[show update]
    end
  end

  get 'invitations/accept', to: 'invitations#accept', as: :accept_invitation

  # Dashboard

  # Global medication workflow entry point

  # Profile

  # Reports

  # Location management

  # Medication management

  # Authentication - Rodauth handles /login, /logout, /create-account via middleware

  # WebAuthn/Passkey routes are handled by Rodauth

  # Progressive Web App assets
  get 'manifest.webmanifest', to: 'pwa#manifest'
  get 'service-worker.js', to: 'pwa#service_worker'

  # Reveal health status on /up that returns 200 if the app boots with no exceptions, otherwise 500.
  # Can be used by load balancers and uptime monitors to verify that the app is live.
  get 'up' => 'rails/health#show', as: :rails_health_check
end
