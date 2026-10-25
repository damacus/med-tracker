import http.cookiejar
import urllib.error
import urllib.parse
import urllib.request
from html.parser import HTMLParser

class Inputs(HTMLParser):
    def __init__(self):
        super().__init__()
        self.fields = {}
    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if tag == 'input' and attrs.get('name'):
            self.fields[attrs['name']] = attrs.get('value', '')

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None

class Client:
    def __init__(self, app):
        self.app = app
        self.base = 'http://localhost:' + ('13000' if app == 'rails' else '15150')
        origin = self.base.replace('http:', 'https:') if app == 'rails' else self.base
        self.headers = {'Origin': origin, 'Referer': origin + '/login'}
        if app == 'rails':
            self.headers['X-Forwarded-Proto'] = 'https'
        self.jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar), NoRedirect())

    def fetch(self, path, data=None, headers=None, timeout=30):
        for cookie in self.jar:
            cookie.secure = False
        req = urllib.request.Request(self.base + path, data=data, headers={**self.headers, **(headers or {})})
        try:
            response = self.opener.open(req, timeout=timeout)
        except urllib.error.HTTPError as error:
            response = error
        return response.code, dict(response.headers), response.read()

    def login(self):
        status, _, body = self.fetch('/login')
        assert status == 200, ('login form', status)
        parser = Inputs(); parser.feed(body.decode())
        fields = {key:value for key,value in parser.fields.items() if key in ['authenticity_token', '_csrf']}
        fields.update(email='persistence@example.test', password='password')
        status, headers, _ = self.fetch('/login', urllib.parse.urlencode(fields).encode())
        assert status in [302,303], ('login', status)
        path = urllib.parse.urlsplit(headers.get('Location', headers.get('location', '/'))).path or '/'
        for _ in range(8):
            status, headers, body = self.fetch(path)
            if status in [302,303]:
                path = urllib.parse.urlsplit(headers.get('Location', headers.get('location'))).path or '/'
                continue
            assert status == 200, (path, status)
            if self.app == 'loco' and path == '/households/choose':
                parser = Inputs(); parser.feed(body.decode())
                values = {key:parser.fields[key] for key in ['authenticity_token','household_id']}
                status, headers, _ = self.fetch('/households/select', urllib.parse.urlencode(values).encode())
                assert status in [302,303], ('selection', status)
                path = urllib.parse.urlsplit(headers.get('Location', headers.get('location'))).path or '/'
                continue
            break
        for _ in range(10):
            status, _, body = self.fetch('/households/persistence-fixture/dashboard')
            assert status == 200 and b'Synthetic' in body, ('dashboard warmup', status)
        self.headers['Cookie'] = '; '.join(cookie.name+'='+cookie.value for cookie in self.jar)
        self.opener = urllib.request.build_opener(NoRedirect())

    def upload(self, archive):
        form = '/households/persistence-fixture/admin/nhs_dmd_import/new' if self.app == 'rails' else '/admin/nhs-dmd'
        action = '/households/persistence-fixture/admin/nhs_dmd_import' if self.app == 'rails' else form
        status, _, body = self.fetch(form)
        assert status == 200, ('admin import form', self.app, status)
        parser = Inputs(); parser.feed(body.decode())
        token = parser.fields['authenticity_token']
        field = 'nhs_dmd_import[release_zip]' if self.app == 'rails' else 'archive'
        boundary = '----benchmarkDmd6577'
        body = (f'--{boundary}\r\nContent-Disposition: form-data; name="authenticity_token"\r\n\r\n{token}\r\n'
                f'--{boundary}\r\nContent-Disposition: form-data; name="{field}"; filename="release.zip"\r\n'
                'Content-Type: application/zip\r\n\r\n').encode() + archive.read_bytes() + f'\r\n--{boundary}--\r\n'.encode()
        status, _, content = self.fetch(action, body, {'Content-Type': 'multipart/form-data; boundary='+boundary}, timeout=7200)
        if status not in [302,303]:
            raise RuntimeError(f'Import upload returned HTTP {status}: {content[:300]!r}')
        return status
