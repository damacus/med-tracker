# MedTracker Project Structure

## Rails Application Layout

### Core Application (`app/`)

- **`controllers/`**: Request handling and business logic
  - `concerns/`: Shared controller modules (e.g., `authentication.rb`)
  - `admin/`: Admin-specific controllers
- **`models/`**: Data models and business logic
  - `concerns/`: Shared model modules
- **`views/`**: Template files organized by controller
  - Uses both ERB templates and Phlex components
- **`components/`**: Phlex component classes
  - `layouts/`: Layout components
  - `dashboard/`, `home/`: Feature-specific components
- **`helpers/`**: View helper methods
- **`jobs/`**: Background job classes
- **`mailers/`**: Email handling
- **`assets/`**: Static assets
  - `stylesheets/`: CSS files organized by feature
  - `images/`: Image assets
  - `config/manifest.js`: Asset pipeline configuration
- **`javascript/`**: Stimulus controllers and JS modules
  - `controllers/`: Stimulus controller files

### Configuration (`config/`)

- **`application.rb`**: Main application configuration
- **`routes.rb`**: URL routing definitions
- **`database.yml`**: Database configuration
- **`deploy.yml`**: Kamal deployment configuration
- **`environments/`**: Environment-specific configs
- **`initializers/`**: Application initialization code
- **`locales/`**: Internationalization files

### Database (`db/`)

- **`migrate/`**: Database migration files
- **`schema.rb`**: Current database schema
- **`seeds.rb`**: Database seed data

### Testing

- **`spec/`**: RSpec test files (primary testing framework)
  - `models/`, `controllers/`, `features/`: Organized by type
  - `system/`: End-to-end system tests
  - `components/`: Component-specific tests
  - `support/`: Test helpers and configuration
- **`test/`**: Minitest files (alternative testing)
  - `system/playwright/`: Playwright-based browser tests

### Documentation & Planning

- **`docs/`**: Project documentation
- **`README.md`**: Project overview and setup instructions
- **`plan.md`**: Development planning notes
- **`relationships.md`**: Data relationship documentation

### Development & Deployment

- **`.github/`**: GitHub Actions and workflows
- **`.kamal/`**: Kamal deployment configuration
- **`.kiro/`**: Kiro AI assistant configuration
- **`.windsurf/`**: Windsurf IDE rules and configuration

## Key Conventions

### File Organization

- Controllers follow RESTful conventions
- Models use singular names, controllers use plural
- Views organized by controller name
- Components use class-based organization with Phlex
- Tests mirror the structure of the code they test

### Naming Patterns

- Models: `User`, `Medicine`, `Prescription`, `MedicationTake`
- Controllers: `UsersController`, `MedicinesController`
- Components: `Dashboard::IndexView`, `Home::IndexView`
- Test files: `*_spec.rb` (RSpec), `*_test.rb` (Minitest)

### CSS Organization

- Feature-based CSS files (e.g., `medicines.css`, `dashboard.css`)
- Shared styles in `shared.css`, `application.css`
- Component-specific styles (e.g., `modal.css`, `card.css`)

### JavaScript Structure

- Stimulus controllers in `app/javascript/controllers/`
- Follow Stimulus naming conventions (`*_controller.js`)
- Use Turbo for SPA-like navigation

## Important Files

- **`Gemfile`**: Ruby dependency management
- **`package.json`**: Node.js dependencies (minimal, mainly Playwright)
- **`.ruby-version`**: Ruby version specification
- **`.rspec`**: RSpec configuration
- **`.rubocop.yml`**: Code style configuration
