# frozen_string_literal: true

Rails.application.routes.draw do
  resource :session
  resources :passwords, param: :token
  # Defines the root path route ("/")
  root 'dashboard#index'

  namespace :admin do
    root to: 'dashboard#index'
    resources :users, only: %i[index new create edit update destroy]
    resources :audit_logs, only: %i[index show]
  end

  # Dashboard
  get 'dashboard', to: 'dashboard#index'

  # Profile
  resource :profile, only: %i[show update]

  # Test route
  get 'test_sheet', to: ->(env) { [200, {'Content-Type' => 'text/html'}, [File.read(Rails.root.join('app/views/test_sheet.html.erb'))]] }

  # Medicine management
  resources :medicines do
    member do
      get :dosages
    end
  end
  get 'medicine-finder', to: 'medicines#finder', as: :medicine_finder

  # Authentication - Rodauth handles /login, /logout, /create-account via middleware
  # Legacy routes kept for backwards compatibility during migration
  get 'sign_up', to: 'users#new' # TODO: Remove in Phase 5
  resources :users, only: %i[new create] # TODO: Remove in Phase 5

  resources :prescriptions do
    resources :take_medicines, only: [:create]
    resources :medication_takes, only: [:create]
  end

  resources :people, except: [:edit] do
    resources :prescriptions, except: [:index] do
      member do
        post :take_medicine
      end
    end

    resources :person_medicines, except: [:index] do
      member do
        post :take_medicine
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
