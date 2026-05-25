# frozen_string_literal: true

Rails.application.routes.draw do
  namespace :api do
    namespace :v1 do
      namespace :auth do
        post :login, to: 'sessions#create'
        post :refresh, to: 'sessions#refresh'
        delete :logout, to: 'sessions#destroy'
      end

      resource :me, only: [:show], controller: 'me'
      resources :people, only: %i[index show]
      resources :locations, only: %i[index show]
      resources :medications, only: %i[index show]
      resources :schedules, only: %i[index show]
      resources :person_medications, only: %i[index show]
      resources :medication_takes, only: [:index]
      resource :notification_preference, only: [:show]
    end
  end

  # Defines the root path route ("/")
  root 'dashboard#index'

  get 'search', to: 'searches#show'

  namespace :admin do
    root to: 'dashboard#index'
    resource :nhs_dmd_import, only: %i[new create]
    resources :users, only: %i[index new create edit update destroy] do
      member do
        post :activate
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
    resources :audit_logs, only: %i[index show]
    resource :settings, only: %i[show update]
  end

  get 'invitations/accept', to: 'invitations#accept', as: :accept_invitation

  # Dashboard
  get 'dashboard', to: 'dashboard#index'

  # Global medication workflow entry point
  get 'add_medication', to: 'medication_workflow#index', as: :add_medication

  # Profile
  resource :profile, only: %i[show update] do
    patch :experiments, on: :member
  end
  delete 'profile/avatar', to: 'profiles#avatar', as: :profile_avatar

  # Reports
  resources :reports, only: %i[index]

  get 'offline', to: 'offline#show'
  get 'offline/snapshot', to: 'offline#snapshot'
  post 'offline/medication_takes', to: 'offline#medication_takes'

  # Location management
  resources :locations do
    resources :location_memberships, only: %i[create destroy]
  end

  # Medication management
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

  # Authentication - Rodauth handles /login, /logout, /create-account via middleware

  # WebAuthn/Passkey routes are handled by Rodauth

  get 'schedules/workflow', to: 'schedules#workflow', as: :schedules_workflow
  post 'schedules/workflow', to: 'schedules#start_workflow', as: :start_schedules_workflow
  get 'schedules/frequency_preview', to: 'schedules#frequency_preview', as: :schedules_frequency_preview
  resources :schedules do
    resources :medication_takes, only: [:create]
  end

  resources :people do
    member do
      get :add_medication
    end

    resources :schedules, except: [:index] do
      member do
        post :take_medication
      end
    end

    resources :person_medications, except: [:index] do
      member do
        patch :reorder
        post :take_medication
      end
    end
    resources :carer_relationships, only: %i[new create], controller: 'people/carer_relationships'
    resources :medication_assignments, only: %i[new create]
  end

  resource :push_subscription, only: %i[create destroy] do
    post :test, on: :collection
  end
  resources :native_device_tokens, only: %i[create destroy]
  resource :notification_preference, only: %i[update]

  # Progressive Web App assets
  get 'manifest.webmanifest', to: 'pwa#manifest'
  get 'service-worker.js', to: 'pwa#service_worker'

  # Reveal health status on /up that returns 200 if the app boots with no exceptions, otherwise 500.
  # Can be used by load balancers and uptime monitors to verify that the app is live.
  get 'up' => 'rails/health#show', as: :rails_health_check
end
