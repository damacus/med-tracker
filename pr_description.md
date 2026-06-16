🎯 **What:**
The application's global search controller processed URLs returned by the backend without verifying the protocol in `hrefAttribute(url)`. This created a risk for Cross-Site Scripting (XSS).

⚠️ **Risk:**
If an attacker could inject a payload with a malicious protocol, such as `javascript:alert(1)`, into the `path` attribute of search results, the unvalidated `hrefAttribute` method would assign it to the DOM's `a` tag `href`. If a user clicked or hit Enter on that result, the malicious script would be executed within the context of their session, enabling the attacker to steal tokens or execute privileged actions.

🛡️ **Solution:**
I modified `hrefAttribute(url)` to securely parse the provided URL using the browser's native `URL` API (`new URL(url, window.location.origin)`). After parsing, the protocol is checked against an allowlist of safe protocols (`http:`, `https:`, `mailto:`, `tel:`). If the protocol is unsafe or parsing fails, the function falls back to a safe `#` URL, preventing execution. Relative URLs are handled correctly because they are parsed relative to `window.location.origin` inside the check while returning the original relative string to the DOM element if safe.
