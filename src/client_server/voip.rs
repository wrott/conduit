use hmac::{Hmac, Mac};
#[cfg(feature = "conduit_bin")]
use rocket::get;
use ruma::api::client::r0::voip::get_turn_server_info;
use ruma::SecondsSinceUnixEpoch;
use sha1::Sha1;
use std::time::{Duration, SystemTime};

use crate::{ConduitResult, database::DatabaseGuard, Ruma};

type HmacSha1 = Hmac<Sha1>;

/// # `GET /_matrix/client/r0/voip/turnServer`
///
/// TODO: Returns information about the recommended turn server.
#[cfg_attr(
feature = "conduit_bin",
get("/_matrix/client/r0/voip/turnServer", data = "<body>")
)]
#[tracing::instrument(skip(body, db))]
pub async fn turn_server_route(
    body: Ruma<get_turn_server_info::Request>,
    db: DatabaseGuard,
) -> ConduitResult<get_turn_server_info::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    let turn_secret = db.globals.turn_secret();

    let (username, password) = if !turn_secret.is_empty() {
        let expiry = SecondsSinceUnixEpoch::from_system_time(
            SystemTime::now() + Duration::from_secs(db.globals.turn_ttl()),
        )
        .expect("time is valid");

        let username: String = format!("{}:{}", expiry.get(), sender_user);

        let mut mac = HmacSha1::new_from_slice(turn_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(username.as_bytes());

        let password: String = base64::encode_config(mac.finalize().into_bytes(), base64::STANDARD);

        (username, password)
    } else {
        (
            db.globals.turn_username().clone(),
            db.globals.turn_password().clone(),
        )
    };

    Ok(get_turn_server_info::Response {
        username,
        password,
        uris: db.globals.turn_uris().to_vec(),
        ttl: Duration::from_secs(db.globals.turn_ttl()),
    }
    .into())
}
