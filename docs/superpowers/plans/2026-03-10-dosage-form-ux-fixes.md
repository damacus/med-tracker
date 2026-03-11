# Dosage Form UX Fixes Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add clickable frequency suggestion badges to the dosage form, and fix the turbo stream bug where newly added dosages don't appear in the wizard list.

**Architecture:** Two independent fixes. (1) Frequency badges: add a row of clickable chip buttons above the frequency text input, backed by a new `frequency-suggestions` Stimulus controller that fills the input on click. (2) Dosage list bug: write a request spec to characterise the turbo_stream response from `DosagesController#create` in wizard mode, confirm the existing render works (or expose the bug), and fix if needed.

**Tech Stack:** Ruby on Rails, Phlex (view components), Stimulus.js, Turbo Streams, RSpec request specs

---

## Chunk 1: Frequency suggestion badges

### Task 1: Stimulus controller for frequency suggestions

**Files:**
- Create: `app/javascript/controllers/frequency_suggestions_controller.js`

- [ ] **Step 1: Write the failing test (JS unit — skip, pure DOM wiring; go straight to implementation)**

  There is no JS unit test harness set up in this project; behaviour will be covered by a request spec that asserts the markup is present. Proceed to implementation.

- [ ] **Step 2: Create the Stimulus controller**

  Create `app/javascript/controllers/frequency_suggestions_controller.js`:

  ```js
  import { Controller } from "@hotwired/stimulus"

  export default class extends Controller {
    static targets = ["input"]

    suggest(event) {
      event.preventDefault()
      if (this.hasInputTarget) {
        this.inputTarget.value = event.currentTarget.dataset.suggestion
        this.inputTarget.focus()
      }
    }
  }
  ```

- [ ] **Step 3: Confirm auto-registration — no change to index.js needed**

  This project uses `eagerLoadControllersFrom("controllers", application)` in `app/javascript/controllers/index.js`. Any file named `*_controller.js` in that directory is auto-registered. **Do not add a manual import** — doing so would double-register the controller and cause a Stimulus error. Simply placing the file in the right directory is sufficient.

- [ ] **Step 4: Commit**

  ```bash
  git add app/javascript/controllers/frequency_suggestions_controller.js
  git commit -m "feat: add frequency-suggestions Stimulus controller"
  ```

---

### Task 2: Render suggestion badges in the dosage form

**Files:**
- Modify: `app/components/dosages/form.rb` — `render_basic_fields` private method, frequency field block (lines 70–79)

- [ ] **Step 1: Write the failing request spec**

  Create `spec/requests/dosages_frequency_suggestions_spec.rb`:

  ```ruby
  # frozen_string_literal: true

  require 'rails_helper'

  RSpec.describe 'Dosage frequency suggestions' do
    fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

    before { sign_in(users(:admin)) }

    it 'renders frequency suggestion badges on the new dosage form' do
      medication = medications(:paracetamol)

      get new_medication_dosage_path(medication)

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Once daily')
      expect(response.body).to include('Every 4&#x2013;6 hours')
      expect(response.body).to include('Every morning')
      expect(response.body).to include('As needed (PRN)')
      # FormField uses Phlex's `mix` to deep-merge `data:` hashes, so the
      # rendered attribute will be data-controller="ruby-ui--form-field frequency-suggestions"
      # (multi-controller Stimulus syntax). Assert with a regex, not a verbatim string.
      expect(response.body).to match(/data-controller="[^"]*frequency-suggestions[^"]*"/)
      expect(response.body).to include('data-action="click->frequency-suggestions#suggest"')
    end
  end
  ```

- [ ] **Step 2: Run the spec to confirm it fails**

  ```bash
  task test
  ```

  Expected: failures on the `include` assertions for badge text and controller attributes.

- [ ] **Step 3: Add the suggestion badges to `render_basic_fields`**

  Open `app/components/dosages/form.rb`. Replace the frequency `FormField` block (lines 70–79):

  ```ruby
  FormField(class: 'mt-4', data: { controller: 'frequency-suggestions' }) do
    FormFieldLabel(for: 'dosage_frequency') do
      plain 'Frequency label'
      span(class: 'text-destructive ml-0.5') { ' *' }
    end
    FormFieldHint { 'Short description, e.g. "Once daily", "Every 4–6 hours"' }
    render_frequency_suggestions
    Input(type: :text, name: 'dosage[frequency]', id: 'dosage_frequency',
          value: dosage.frequency, required: true,
          placeholder: 'Once daily',
          data: { 'frequency-suggestions-target': 'input' })
  end
  ```

  Then add a new private method `render_frequency_suggestions` to the same class (after `render_basic_fields`):

  ```ruby
  FREQUENCY_SUGGESTIONS = [
    'Once daily',
    'Twice daily',
    'Three times daily',
    'Every 4 hours',
    'Every 4–6 hours',
    'Every 6 hours',
    'Every 8 hours',
    'Every 12 hours',
    'Every morning',
    'Every night',
    'As needed (PRN)'
  ].freeze

  def render_frequency_suggestions
    div(class: 'flex flex-wrap gap-1.5 mt-1 mb-2') do
      FREQUENCY_SUGGESTIONS.each do |suggestion|
        button(
          type: 'button',
          data: {
            action: 'click->frequency-suggestions#suggest',
            suggestion: suggestion
          },
          class: 'inline-flex items-center rounded-full border border-slate-200 bg-white ' \
                 'px-2.5 py-0.5 text-xs font-medium text-slate-600 shadow-sm ' \
                 'hover:bg-slate-50 hover:border-slate-300 cursor-pointer transition-colors'
        ) { suggestion }
      end
    end
  end
  ```

  > **Note:** `FormField` inherits from `RubyUI::Base` which merges attributes via Phlex's `mix`. `mix` deep-merges `data:` hashes and space-joins the `controller:` subkey, so passing `data: { controller: 'frequency-suggestions' }` will produce `data-controller="ruby-ui--form-field frequency-suggestions"` — valid Stimulus multi-controller syntax. This is intentional and correct; no extra wrapper `div` is needed.

- [ ] **Step 4: Run the spec**

  ```bash
  task test
  ```

  Expected: the new spec passes. Existing specs continue to pass.

- [ ] **Step 5: Commit**

  ```bash
  git add app/components/dosages/form.rb \
          spec/requests/dosages_frequency_suggestions_spec.rb
  git commit -m "feat: add frequency suggestion badges to dosage form"
  ```

---

## Chunk 2: Fix turbo stream — dosage list not updating after add

### Task 3: Request spec for wizard dosage creation turbo stream

**Files:**
- Create: `spec/requests/dosages_wizard_spec.rb`

The controller already has turbo_stream handling. The spec will confirm whether the response:
- Contains the newly added `DosageRow` HTML (amount + unit)
- Contains the refreshed `DosageFormFrame` (empty form for the next entry)

- [ ] **Step 1: Write the spec**

  Create `spec/requests/dosages_wizard_spec.rb`:

  ```ruby
  # frozen_string_literal: true

  require 'rails_helper'

  RSpec.describe 'Wizard dosage creation' do
    fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

    before { sign_in(users(:admin)) }

    let(:medication) { medications(:paracetamol) }

    it 'returns turbo_stream that appends the new dosage row and resets the form' do
      post medication_dosages_path(medication),
           params: {
             wizard: 'true',
             dosage: {
               amount: '2.5',
               unit: 'ml',
               frequency: 'Once daily',
               default_for_adults: '1'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.content_type).to include('text/vnd.turbo-stream.html')

      body = response.body
      expect(body).to include('turbo-stream')
      expect(body).to include('action="append"')
      expect(body).to include('target="dosage-list"')
      expect(body).to include('2.5')
      expect(body).to include('ml')
      expect(body).to include('action="replace"')
      expect(body).to include('target="dosage-form"')
    end

    it 'redirects to medication page without turbo stream header' do
      post medication_dosages_path(medication),
           params: {
             wizard: 'true',
             dosage: {
               amount: '5',
               unit: 'ml',
               frequency: 'Twice daily'
             }
           }

      expect(response).to redirect_to(medication_path(medication))
    end
  end
  ```

- [ ] **Step 2: Run the spec**

  ```bash
  task test
  ```

  **If the spec passes:** the turbo_stream rendering is working and the issue is client-side (e.g. a missing `<turbo-frame id="dosage-form">` wrapper or a JS error). Skip to Task 4 to investigate.

  **If the spec fails** on the content assertions (no `action="append"` or no `2.5` in body): the Phlex component isn't rendering inside the turbo_stream helper — proceed to Task 4 to fix the controller.

- [ ] **Step 3: Commit the spec regardless**

  ```bash
  git add spec/requests/dosages_wizard_spec.rb
  git commit -m "test: add request spec for wizard dosage creation turbo stream"
  ```

---

### Task 4: Fix turbo stream rendering (if spec failed in Task 3)

> Skip this task if the spec from Task 3 already passes.

**Files:**
- Modify: `app/controllers/dosages_controller.rb` — `create` action

The issue is that `turbo_stream.append(target, phlex_component)` may not invoke Phlex's `render_in` pipeline correctly in older versions of turbo-rails. The fix is to render each component via a block so the view context is explicitly passed.

- [ ] **Step 1: Update the `create` action turbo_stream block**

  In `app/controllers/dosages_controller.rb`, change lines 24–31 from:

  ```ruby
  format.turbo_stream do
    dosage_row = Components::Medications::Wizard::DosageRow.new(dosage: @dosage)
    form_frame = Components::Medications::Wizard::DosageFormFrame.new(medication: @medication)
    render turbo_stream: [
      turbo_stream.append('dosage-list', dosage_row),
      turbo_stream.replace('dosage-form', form_frame)
    ]
  end
  ```

  to:

  ```ruby
  format.turbo_stream do
    render turbo_stream: [
      turbo_stream.append('dosage-list') {
        render Components::Medications::Wizard::DosageRow.new(dosage: @dosage)
      },
      turbo_stream.replace('dosage-form') {
        render Components::Medications::Wizard::DosageFormFrame.new(medication: @medication)
      }
    ]
  end
  ```

- [ ] **Step 2: Run the spec**

  ```bash
  task test
  ```

  Expected: the `dosages_wizard_spec.rb` spec now passes. All other specs continue to pass.

- [ ] **Step 3: Commit**

  ```bash
  git add app/controllers/dosages_controller.rb
  git commit -m "fix: use block form for turbo_stream Phlex rendering in wizard dosage create"
  ```

---

### Task 5: Full test run and push

- [ ] **Step 1: Run all tests**

  ```bash
  task test
  ```

  Expected: all green.

- [ ] **Step 2: Run linter**

  ```bash
  task rubocop
  ```

  Fix any offences (usually frozen_string_literal, trailing whitespace).

- [ ] **Step 3: Push**

  ```bash
  git pull --rebase
  git push
  ```
