# frozen_string_literal: true

Rails.application.routes.draw do
  # Defines the root path route ("/")
  root 'dashboard#index'

  namespace :admin do
    root to: 'dashboard#index'
    resources :users, only: %i[index new create edit update destroy] do
      member do
        post :activate
        post :verify
      end
    end
    resources :invitations, only: %i[index create]
    resources :carer_relationships, only: %i[index new create destroy] do
      member do
        post :activate
      end
    end
    resources :audit_logs, only: %i[index show]
  end

  get 'invitations/accept', to: 'invitations#accept', as: :accept_invitation

  # Dashboard
  get 'dashboard', to: 'dashboard#index'

  # Profile
  resource :profile, only: %i[show update]

  # Reports
  resources :reports, only: %i[index]

  # Location management
  resources :locations do
    resources :location_memberships, only: %i[create destroy]
  end

  # Medication management
  resources :medications do
    member do
      get :dosages
      patch :refill
      patch :mark_as_ordered
      patch :mark_as_received
    end
  end
  get 'medication-finder', to: 'medications#finder', as: :medication_finder
  get 'medication-finder/search', to: 'medications#search', as: :medication_finder_search

  # Authentication - Rodauth handles /login, /logout, /create-account via middleware

  # WebAuthn/Passkey routes are handled by Rodauth

  resources :schedules do
    resources :medication_takes, only: [:create]
  end

  resources :people do
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
  end

  # Progressive Web App assets
  get 'manifest.webmanifest', to: 'pwa#manifest'
  get 'service-worker.js', to: 'pwa#service_worker'

  # Reveal health status on /up that returns 200 if the app boots with no exceptions, otherwise 500.
  # Can be used by load balancers and uptime monitors to verify that the app is live.
  get 'up' => 'rails/health#show', as: :rails_health_check
end
