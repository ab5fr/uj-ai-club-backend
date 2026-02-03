use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};

use crate::AppState;

pub async fn google_auth_init(State(state): State<AppState>) -> impl IntoResponse {
    use oauth2::basic::BasicClient;
    use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};

    // Create OAuth client
    let client = BasicClient::new(
        ClientId::new(state.oauth_config.client_id.clone()),
        Some(ClientSecret::new(state.oauth_config.client_secret.clone())),
        AuthUrl::new(state.oauth_config.auth_url.clone())
            .expect("Invalid authorization endpoint URL"),
        Some(
            TokenUrl::new(state.oauth_config.token_url.clone())
                .expect("Invalid token endpoint URL"),
        ),
    )
    .set_redirect_uri(
        RedirectUrl::new(state.oauth_config.redirect_uri.clone()).expect("Invalid redirect URL"),
    );

    // Generate authorization URL
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    Redirect::temporary(auth_url.as_str())
}
