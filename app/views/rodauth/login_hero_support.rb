# frozen_string_literal: true

module Views
  module Rodauth
    module LoginHeroSupport
      private

      def render_background_atmosphere
        div(class: 'fixed inset-0 overflow-hidden pointer-events-none') do
          div(class: 'absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(200,122,60,0.18),transparent_30%),radial-gradient(circle_at_82%_18%,rgba(59,130,246,0.12),transparent_22%),radial-gradient(circle_at_bottom_right,rgba(47,133,90,0.18),transparent_30%)]')
          div(class: 'absolute left-[6%] top-[12%] h-64 w-64 rounded-full border border-white/40 bg-white/20 blur-3xl')
          div(class: 'absolute bottom-[10%] right-[4%] h-80 w-80 rounded-full bg-[rgba(23,37,84,0.08)] blur-3xl')
          div(class: 'absolute inset-y-0 left-1/2 hidden w-px -translate-x-1/2 bg-[linear-gradient(180deg,transparent,rgba(24,24,27,0.08),transparent)] lg:block')
        end
      end

      def render_brand_panel
        section(class: 'relative hidden overflow-hidden rounded-[2.75rem] border border-black/10 bg-[linear-gradient(160deg,rgba(24,24,27,0.96),rgba(39,39,42,0.92)_52%,rgba(82,55,29,0.88))] p-8 text-stone-50 shadow-[0_45px_100px_-35px_rgba(24,24,27,0.7)] lg:flex lg:min-h-[720px] lg:flex-col') do
          div(class: 'absolute inset-0 opacity-60') do
            div(class: 'absolute inset-y-10 left-10 w-px bg-[linear-gradient(180deg,transparent,rgba(255,255,255,0.25),transparent)]')
            div(class: 'absolute right-10 top-10 h-28 w-28 rounded-full border border-white/15')
            div(class: 'absolute bottom-16 right-16 h-48 w-48 rounded-full bg-[radial-gradient(circle,rgba(255,255,255,0.14),transparent_62%)]')
            div(class: 'absolute inset-x-12 bottom-12 h-px bg-[linear-gradient(90deg,transparent,rgba(255,255,255,0.22),transparent)]')
          end
          div(class: 'relative flex h-full flex-col justify-between') do
            render_brand_identity
            render_brand_story
          end
        end
      end

      def render_brand_identity
        div(class: 'space-y-8') do
          div(class: 'inline-flex items-center gap-3 rounded-full border border-white/15 bg-white/8 px-4 py-2 text-[0.68rem] font-bold uppercase tracking-[0.34em] text-stone-200/90 backdrop-blur') do
            span(class: 'inline-flex h-2 w-2 rounded-full bg-emerald-300 shadow-[0_0_0_6px_rgba(110,231,183,0.12)]')
            plain t('sessions.login.tagline')
          end
          div(class: 'space-y-5') do
            div(class: 'flex h-16 w-16 items-center justify-center rounded-[1.6rem] border border-white/12 bg-white/10 shadow-[inset_0_1px_0_rgba(255,255,255,0.18)] backdrop-blur') do
              render Components::Icons::Pill.new(size: 28)
            end
            Heading(level: 1, size: '8', class: 'max-w-lg font-black leading-[0.94] tracking-[-0.05em] text-stone-50') do
              plain 'Clinical calm,'
              br
              plain 'without the waiting room.'
            end
            Text(size: '3', class: 'max-w-xl text-base leading-7 text-stone-200/78') do
              plain 'A sign-in experience for patients and caregivers that feels secure, immediate, and clearly medical.'
            end
          end
        end
      end

      def render_brand_story
        div(class: 'space-y-10') do
          div(class: 'grid gap-4 sm:grid-cols-3') do
            brand_metric('Passkeys ready', 'Biometric sign-in with conditional autofill')
            brand_metric('Trusted routine', 'Medication access with fewer interruptions')
            brand_metric('Quiet fallback', 'Password, OIDC, and recovery flows stay intact')
          end

          div(class: 'grid gap-4 sm:grid-cols-[auto_1fr] sm:items-start') do
            div(class: 'inline-flex h-12 w-12 items-center justify-center rounded-2xl border border-white/15 bg-white/10 text-stone-100 backdrop-blur') do
              render Components::Icons::Activity.new(size: 22)
            end
            div(class: 'space-y-3') do
              p(class: 'text-[0.68rem] font-bold uppercase tracking-[0.34em] text-stone-300/75') { 'Live login surface' }
              p(class: 'max-w-lg text-sm leading-7 text-stone-200/80') do
                plain 'Designed to make the primary action obvious, keep support links close, and treat passkeys as first-class rather than an advanced option.'
              end
            end
          end
        end
      end

      def brand_metric(title, detail)
        div(class: 'rounded-[1.75rem] border border-white/12 bg-white/8 p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.12)] backdrop-blur') do
          p(class: 'text-sm font-bold text-stone-50') { title }
          p(class: 'mt-2 text-sm leading-6 text-stone-300/78') { detail }
        end
      end
    end
  end
end
