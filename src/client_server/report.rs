use crate::{
    database::{admin::AdminCommand, DatabaseGuard},
    ConduitResult, Error, Ruma,
};
use ruma::{
    api::client::{error::ErrorKind, r0::room::report_content},
    events::room::message,
    int,
};

#[cfg(feature = "conduit_bin")]
use rocket::{http::RawStr, post};

/// # `POST /_matrix/client/r0/rooms/{roomId}/report/{eventId}`
///
/// Reports an inappropriate event to homeserver admins
///
#[cfg_attr(
    feature = "conduit_bin",
    post("/_matrix/client/r0/rooms/<_>/report/<_>", data = "<body>")
)]
#[tracing::instrument(skip(db, body))]
pub async fn report_event_route(
    db: DatabaseGuard,
    body: Ruma<report_content::Request<'_>>,
) -> ConduitResult<report_content::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    let pdu = match db.rooms.get_pdu(&body.event_id)? {
        Some(pdu) => pdu,
        _ => {
            return Err(Error::BadRequest(
                ErrorKind::InvalidParam,
                "Invalid Event ID",
            ))
        }
    };

    if body.score > int!(0) || body.score < int!(-100) {
        return Err(Error::BadRequest(
            ErrorKind::InvalidParam,
            "Invalid score, must be within 0 to -100",
        ));
    };

    if body.reason.chars().count() > 250 {
        return Err(Error::BadRequest(
            ErrorKind::InvalidParam,
            "Reason too long, should be 250 characters or fewer",
        ));
    };

    db.admin.send(AdminCommand::SendMessage(
        message::RoomMessageEventContent::text_html(
            format!(
                "Report received from: {}\n\n\
                Event ID: {}\n\
                Room ID: {}\n\
                Sent By: {}\n\n\
                Report Score: {}\n\
                Report Reason: {}",
                sender_user, pdu.event_id, pdu.room_id, pdu.sender, body.score, body.reason
            ),
            format!(
                "<details><summary>Report received from: <a href=\"https://matrix.to/#/{0}\">{0}\
                </a></summary><ul><li>Event Info<ul><li>Event ID: <code>{1}</code>\
                <a href=\"https://matrix.to/#/{2}/{1}\">????</a></li><li>Room ID: <code>{2}</code>\
                </li><li>Sent By: <a href=\"https://matrix.to/#/{3}\">{3}</a></li></ul></li><li>\
                Report Info<ul><li>Report Score: {4}</li><li>Report Reason: {5}</li></ul></li>\
                </ul></details>",
                sender_user,
                pdu.event_id,
                pdu.room_id,
                pdu.sender,
                body.score,
                RawStr::new(&body.reason).html_escape()
            ),
        ),
    ));

    db.flush()?;

    Ok(report_content::Response {}.into())
}
