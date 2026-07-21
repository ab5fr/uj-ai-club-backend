import json
import jwt
import os
import urllib.error
import urllib.request

from jupyterhub.auth import Authenticator
from traitlets import Bool, Unicode

def _validate_token_with_backend(token: str) -> dict | None:
    backend_url = os.environ.get("BACKEND_URL", "http://backend:8000").rstrip("/")
    secret = os.environ.get("GRADING_SERVICE_SECRET", "")
    if not secret:
        return None

    req = urllib.request.Request(
        f"{backend_url}/internal/jupyterhub/validate-token",
        data=json.dumps({"token": token}).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "X-Grading-Service-Secret": secret,
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, json.JSONDecodeError):
        return None

class JWTAuthenticator(Authenticator):

    jwt_secret = Unicode(
        config=True,
        help="The secret key used to decode JWT tokens (must match the main app's JWT_SECRET)",
    )

    auto_login = Bool(
        True,
        config=True,
        help="Automatically login users with valid tokens",
    )

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.jwt_secret = os.environ.get("JWT_SECRET", "")
        if not self.jwt_secret:
            self.log.warning(
                "JWT_SECRET environment variable not set - authentication will fail"
            )

    async def authenticate(self, handler, data):
        token = handler.get_argument("token", None)

        if not token:
            auth_header = handler.request.headers.get("Authorization", "")
            if auth_header.startswith("Bearer "):
                token = auth_header[7:]

        if not token:
            self.log.warning("No JWT token provided")
            return None

        try:
            payload = jwt.decode(token, self.jwt_secret, algorithms=["HS256"])
        except jwt.ExpiredSignatureError:
            self.log.warning("JWT token has expired")
            return None
        except jwt.InvalidTokenError as e:
            self.log.warning(f"Invalid JWT token: {e}")
            return None

        purpose = payload.get("purpose")
        if purpose not in ("jupyterhub_sso", "jupyterhub_admin"):
            self.log.warning("Invalid token purpose")
            return None

        username = payload.get("username")
        if not username:
            self.log.warning("No username in token")
            return None

        validation = _validate_token_with_backend(token)
        if not validation or not validation.get("allowed"):
            message = (validation or {}).get("message", "Token rejected by API")
            self.log.warning(f"Backend token validation failed: {message}")
            return None

        is_admin = bool(validation.get("admin")) and purpose == "jupyterhub_admin"
        self.log.info(f"Successfully authenticated user: {username} (admin={is_admin})")

        return {
            "name": username,
            "admin": is_admin,
            "auth_state": {
                "user_id": payload.get("sub"),
                "token": token,
                "purpose": purpose,
            },
        }

    def get_handlers(self, app):
        return [(r"/login", TokenLoginHandler)]

    async def pre_spawn_start(self, user, spawner):
        auth_state = await user.get_auth_state()
        if auth_state:
            spawner.environment["AICLUB_USER_ID"] = auth_state.get("user_id", "")

from jupyterhub.handlers import BaseHandler

class TokenLoginHandler(BaseHandler):

    async def get(self):
        token = self.get_argument("token", None)
        next_url = self.get_argument("next", "/")

        if not token:
            main_app_url = os.environ.get("MAIN_APP_URL", "https://aiclub-uj.com")
            self.redirect(f"{main_app_url}/login?redirect=jupyterhub")
            return

        auth_user = await self.authenticator.authenticate(self, None)
        if not auth_user:
            self.set_status(401)
            self.write(
                "Authentication failed. Please log in through the main application."
            )
            return

        user = await self.login_user(auth_user)
        if user:
            self.redirect(next_url)
        else:
            self.set_status(403)
            self.write(
                "Your account is not permitted to use JupyterHub. "
                "Please contact an administrator."
            )
