const responseForm = document.getElementById('oauth-response-form');
if (responseForm instanceof HTMLFormElement) HTMLFormElement.prototype.submit.call(responseForm);
