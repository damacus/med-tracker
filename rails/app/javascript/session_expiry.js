const LOGIN_PATH = "/login"

function responseObject(responseLike) {
  return responseLike?.response || responseLike
}

function sessionRedirectLocation(responseLike) {
  const response = responseObject(responseLike)
  if (!response?.url) return null

  const url = new URL(response.url, window.location.origin)
  if (url.origin !== window.location.origin) return null
  if (url.pathname !== LOGIN_PATH) return null
  if (window.location.pathname === LOGIN_PATH) return null

  return url.toString()
}

function redirectToLogin(responseLike) {
  const location = sessionRedirectLocation(responseLike)
  if (!location) return false

  window.location.assign(location)
  return true
}

function handleBeforeFetchResponse(event) {
  const response = event.detail.fetchResponse?.response
  if (!response) return

  if (redirectToLogin(response)) {
    event.preventDefault()
  }
}

function handleFrameMissing(event) {
  if (redirectToLogin(event.detail.response)) {
    event.preventDefault()
  }
}

function registerFrameListeners() {
  document.querySelectorAll("turbo-frame").forEach((frame) => {
    if (frame.dataset.sessionExpiryListenersRegistered === "true") return

    frame.addEventListener("turbo:before-fetch-response", handleBeforeFetchResponse)
    frame.addEventListener("turbo:frame-missing", handleFrameMissing)
    frame.dataset.sessionExpiryListenersRegistered = "true"
  })
}

document.addEventListener("turbo:before-fetch-response", handleBeforeFetchResponse)
document.addEventListener("turbo:frame-missing", handleFrameMissing)
document.addEventListener("turbo:load", registerFrameListeners)

registerFrameListeners()
